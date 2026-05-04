use std::sync::Arc;

use chrono::Utc;
use color_eyre::eyre::{
    Error,
    eyre,
};
use futures_util::TryStreamExt;
use num_complex::Complex;
use num_traits::Zero;
use rtlsdr_async::{
    Gain,
    Iq,
    RtlSdr,
    Samples,
};
use stolas_core::Frame;
use tokio::{
    sync::{
        broadcast,
        mpsc,
        oneshot,
    },
    task::JoinHandle,
};
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::{
    config::{
        AntennaConfig,
        ReceiverConfig,
    },
    util::linear_to_db,
};

#[derive(Debug)]
pub struct ReceiverOptions {
    pub center_frequency: u32,
    pub sample_rate: u32,
    pub tuner_gain: f32,
}

#[derive(Debug)]
pub struct PipelineOptions {
    pub window_size: usize,
    pub average_size: usize,
}

/// # Note
///
/// This is a somewhat thin wrapper around a receiver and the processing
/// pipeline, and it could easily just be handled by the Station struct. We want
/// to keep it separate to make it easier to support multiple antennas.
#[derive(Clone, Debug)]
pub struct Antenna {
    command_sender: mpsc::Sender<Command>,
    weak_frame_sender: broadcast::WeakSender<Frame>,
    shared: Arc<Shared>,
}

#[derive(Debug)]
#[allow(unused)]
struct Shared {
    antenna_config: AntennaConfig,
    drop_guard: DropGuard,
    join_handle: JoinHandle<()>,
}

impl Antenna {
    pub fn new(antenna_config: AntennaConfig) -> Result<Self, Error> {
        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();

        let (command_sender, command_receiver) = mpsc::channel(16);
        let (frame_sender, _frame_receiver) = broadcast::channel(128);
        let weak_frame_sender = frame_sender.downgrade();

        let antenna_task =
            AntennaTask::new(antenna_config.clone(), command_receiver, frame_sender)?;

        let join_handle = tokio::spawn(async move {
            tracing::debug!("starting antenna task");

            match antenna_task.run().await {
                Ok(()) => {
                    tracing::debug!("antenna task stopped");
                }
                Err(error) => {
                    tracing::error!(%error, "antenna task failed");
                }
            }
        });

        Ok(Self {
            command_sender,
            weak_frame_sender,
            shared: Arc::new(Shared {
                antenna_config,
                drop_guard,
                join_handle,
            }),
        })
    }

    pub fn antenna_config(&self) -> &AntennaConfig {
        &self.shared.antenna_config
    }

    pub fn receiver_options(&self) -> &ReceiverOptions {
        // rename? receiver -> frontend
        // options -> settings?
        todo!();
    }

    pub async fn start(
        &self,
        receiver_options: ReceiverOptions,
        pipeline_options: PipelineOptions,
    ) -> Result<(), Error> {
        let (result_sender, result_receiver) = oneshot::channel();

        self.command_sender
            .send(Command::Start {
                receiver_options,
                pipeline_options,
                result_sender,
            })
            .await
            .expect("command receiver closed");

        result_receiver.await.expect("antenna task disn't reply")
    }

    pub async fn stop(&self) {
        self.command_sender
            .send(Command::Stop)
            .await
            .expect("command receiver closed");
    }

    pub fn frames(&self) -> Frames {
        let receiver = self
            .weak_frame_sender
            .upgrade()
            .expect("event sender closed")
            .subscribe();

        Frames { receiver }
    }
}

#[derive(Debug)]
pub struct Frames {
    receiver: broadcast::Receiver<Frame>,
}

impl Frames {
    pub async fn next_frame(&mut self) -> Frame {
        loop {
            match self.receiver.recv().await {
                Ok(frame) => return frame,
                Err(_) => {
                    // for now we decided to ignore lagged streams. it just
                    // complicates things.
                    //
                    // if we really want to return that, but also keep this
                    // simpler stream interface, we could
                    // add a method that converts this stream to an identical
                    // type, which returns an enum that includes the lag events.
                }
            }
        }
    }
}

#[derive(Debug)]
enum Command {
    Start {
        receiver_options: ReceiverOptions,
        pipeline_options: PipelineOptions,
        result_sender: oneshot::Sender<Result<(), Error>>,
    },
    Stop,
}

struct AntennaTask {
    receiver: Receiver,
    command_receiver: mpsc::Receiver<Command>,
    frame_sender: broadcast::Sender<Frame>,
    pipeline: Option<Pipeline>,
}

impl AntennaTask {
    fn new(
        antenna_config: AntennaConfig,
        command_receiver: mpsc::Receiver<Command>,
        frame_sender: broadcast::Sender<Frame>,
    ) -> Result<Self, Error> {
        let receiver = Receiver::new(antenna_config.receiver)?;

        Ok(Self {
            receiver,
            command_receiver,
            frame_sender,
            pipeline: None,
        })
    }

    async fn run(mut self) -> Result<(), Error> {
        let mut frames = vec![];

        async fn process_next_chunk(
            pipeline: &mut Option<Pipeline>,
            frames: &mut Vec<Frame>,
        ) -> Result<(), Error> {
            if let Some(pipeline) = pipeline {
                let chunk = pipeline
                    .samples
                    .try_next()
                    .await?
                    .expect("samples: end of stream");

                assert!(frames.is_empty());
                pipeline.push_samples(chunk.as_ref(), frames);

                Ok(())
            }
            else {
                std::future::pending().await
            }
        }

        loop {
            tokio::select! {
                command = self.command_receiver.recv() => {
                    let Some(command) = command else { break; };
                    self.handle_command(command).await?;
                }
                result = process_next_chunk(&mut self.pipeline, &mut frames) => {
                    result?;

                    for frame in frames.drain(..) {
                        let stats = frame.stats();
                        tracing::debug!(
                            serial = frame.serial,
                            "Full frame: min={} dB, max={} dB, average={} dB",
                            linear_to_db(stats.min),
                            linear_to_db(stats.max),
                            linear_to_db(stats.average),
                        );

                        // note: we ignore if the frame channel is closed. when the frame channel is
                        // closed, the command channel is closed as well, which will cause the main loop
                        // to exit.
                        let _ = self.frame_sender.send(frame);
                    }

                }
            };
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: Command) -> Result<(), Error> {
        match command {
            Command::Start {
                receiver_options,
                pipeline_options,
                result_sender,
            } => {
                let result = self.start(receiver_options, pipeline_options).await;
                let _ = result_sender.send(result);
            }
            Command::Stop => {
                self.pipeline = None;
            }
        }

        Ok(())
    }

    async fn start(
        &mut self,
        receiver_options: ReceiverOptions,
        pipeline_options: PipelineOptions,
    ) -> Result<(), Error> {
        tracing::info!(?receiver_options, ?pipeline_options, "Start receiving");

        self.receiver.configure(&receiver_options).await?;
        let samples = self.receiver.samples().await?;
        self.pipeline = Some(Pipeline::new(samples, &pipeline_options));

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Receiver {
    sdr: RtlSdr,
}

impl Receiver {
    pub fn new(receiver_config: ReceiverConfig) -> Result<Self, Error> {
        let index = if let Some(find_serial) = &receiver_config.serial {
            let info = rtlsdr_async::devices()
                .find(|info| info.serial().is_some_and(|serial| serial == find_serial))
                .ok_or_else(|| eyre!("Could not find RTL-SDR with serial '{find_serial}'"))?;

            let index = info.index();
            tracing::info!(serial = ?find_serial, index, "Found RTL-SDR by serial");
            index
        }
        else {
            0
        };

        let sdr = RtlSdr::open(index)?;

        if receiver_config.bias_tee {
            tracing::warn!("Bias-Tee is currently not supported");
        }

        Ok(Self { sdr })
    }

    pub async fn configure(&self, receiver_options: &ReceiverOptions) -> Result<(), Error> {
        // configure SDR
        self.sdr
            .set_center_frequency(receiver_options.center_frequency)
            .await?;
        self.sdr
            .set_sample_rate(receiver_options.sample_rate)
            .await?;

        // select closest gain. rtlsdr-async can do this, but we want to log this
        //
        // todo: we should enforce that only available gains can be used.
        let gain = {
            let available_gains = self.sdr.get_tuner_gains();
            let target_gain = (10.0 * receiver_options.tuner_gain) as i32;
            let (gain_index, gain_value) = available_gains
                .iter()
                .enumerate()
                .min_by_key(|(_i, gain)| (**gain - target_gain).abs())
                .unwrap();

            tracing::debug!(
                ?target_gain,
                ?available_gains,
                ?gain_index,
                gain_value,
                "selecting gain"
            );

            Gain::ManualIndex(gain_index)
        };
        self.sdr.set_tuner_gain(gain).await?;

        Ok(())
    }

    pub async fn samples(&self) -> Result<Samples<Iq>, Error> {
        Ok(self.sdr.samples().await?)
    }
}

#[derive(Debug)]
struct Pipeline {
    window_size: usize,
    samples: Samples<Iq>,
    sample_buffer: Vec<Complex<f32>>,
    freq_buffer: Vec<f32>,
    fft: Fft,
    average: Average,
    num_frames: usize,
}

impl Pipeline {
    pub fn new(samples: Samples<Iq>, options: &PipelineOptions) -> Self {
        let sample_buffer = Vec::with_capacity(options.window_size);
        let freq_buffer = Vec::with_capacity(options.window_size);
        let fft = Fft::new(options.window_size);
        let average = Average::new(options.window_size, options.average_size);

        Self {
            window_size: options.window_size,
            samples,
            sample_buffer,
            freq_buffer,
            fft,
            average,
            num_frames: 0,
        }
    }

    pub fn push_samples(&mut self, samples: &[Iq], frames: &mut Vec<Frame>) {
        // convert samples to magnitude and buffer them
        for sample in samples {
            self.sample_buffer.push(Complex::from(*sample));

            if self.sample_buffer.len() == self.window_size {
                // fft samples
                self.fft.process(&mut self.sample_buffer);

                // take magnitude of spectrum
                assert!(self.freq_buffer.is_empty());
                for i in 0..self.window_size {
                    // calculate power
                    //
                    // https://www.tek.com/en/blog/calculating-rf-power-iq-samples
                    //
                    // though the input is not in Volt, so we don't need to divide by 50 Ohm. The
                    // calculated power will just be in an arbitrary scale.

                    let p_rms = self.sample_buffer[i].norm_sqr();
                    self.freq_buffer.push(p_rms);
                }

                // average spectra
                if let Some(average) = self.average.push(&self.freq_buffer) {
                    // todo: we could convert to dB here, if we wanted to
                    //
                    // https://dsp.stackexchange.com/questions/19615/converting-raw-i-q-to-db
                    //
                    // average[i] = 10.0 * average[i].log10();
                    //
                    // since our input samples are scaled to [-1, 1], this would be dbFFS
                    //
                    // Kevin's this stack overflow answer also touches on "power spectral density".
                    // For this one would divide the power by bin width before converting to dB.
                    // we need to check the Radio Astronomy book what we actually need here.
                    //
                    // but anyway, the output array measures the power (linear, arbitrary scale)
                    // received over `average_size * window_size / sample_rate` seconds over
                    // frequency bins of `sample_rate / window_size` Hz width.

                    frames.push(Frame {
                        serial: self.num_frames.try_into().unwrap(),
                        timestamp: Utc::now(),
                        bins: average,
                    });
                    self.num_frames += 1;
                }

                self.freq_buffer.clear();
                self.sample_buffer.clear();
            }
            else {
                assert!(self.sample_buffer.len() < self.window_size)
            }
        }
    }
}

#[derive(derive_more::Debug)]
struct Fft {
    #[debug(skip)]
    fft: Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    window_size: usize,
}

impl Fft {
    pub fn new(window_size: usize) -> Self {
        let mut fft_planner = rustfft::FftPlanner::new();
        let fft = fft_planner.plan_fft_forward(window_size);
        let scratch = vec![Complex::zero(); fft.get_inplace_scratch_len()];

        Self {
            fft,
            scratch,
            window_size,
        }
    }

    pub fn process(&mut self, buffer: &mut [Complex<f32>]) {
        self.fft.process_with_scratch(buffer, &mut self.scratch);
        for x in buffer {
            *x /= self.window_size as f32;
        }
    }
}

#[derive(Debug)]
struct Average {
    size: usize,
    count: usize,
    sum: Vec<f32>,
}

impl Average {
    pub fn new(window_size: usize, average_size: usize) -> Self {
        Self {
            size: average_size,
            count: 0,
            sum: vec![0.0; window_size],
        }
    }

    pub fn push(&mut self, values: &[f32]) -> Option<Box<[f32]>> {
        assert_eq!(values.len(), self.sum.len());

        for (s, x) in self.sum.iter_mut().zip(values.iter()) {
            *s += *x;
        }

        self.count += 1;

        assert!(self.count <= self.size);
        if self.count == self.size {
            // finish average by dividing sum by average size
            let output: Box<[f32]> = self.sum.iter().map(|sum| *sum / self.size as f32).collect();

            // reset accumulator
            self.sum.fill(0.0);
            self.count = 0;

            Some(output)
        }
        else {
            None
        }
    }
}
