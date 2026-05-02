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
    Chunk,
    Gain,
    Iq,
    RtlSdr,
    Samples,
};
use stolas_core::{
    AntennaConfig,
    Frame,
    ProcessingConfig,
    SdrConfig,
};
use tokio::{
    sync::{
        broadcast,
        mpsc,
    },
    task::JoinHandle,
};
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

#[derive(Clone, Debug)]
pub struct Antenna {
    command_sender: mpsc::Sender<Command>,
    frame_channel: broadcast::WeakSender<Frame>,
    #[allow(unused)]
    shared: Arc<Shared>,
}

#[derive(Debug)]
#[allow(unused)]
struct Shared {
    drop_guard: DropGuard,
    join_handle: JoinHandle<()>,
}

impl Antenna {
    pub async fn new(config: AntennaConfig) -> Result<Self, Error> {
        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();

        let (command_sender, command_receiver) = mpsc::channel(16);
        let (frame_sender, _frame_receiver) = broadcast::channel(128);
        let frame_channel = frame_sender.downgrade();

        let sampling_task = SamplingTask::new(config, command_receiver, frame_sender).await?;

        let join_handle = tokio::spawn(async move {
            tracing::debug!("starting antenna task");

            match sampling_task.run().await {
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
            frame_channel,
            shared: Arc::new(Shared {
                drop_guard,
                join_handle,
            }),
        })
    }

    pub async fn reconfigure(&self, config: AntennaConfig) {
        self.command_sender
            .send(Command::Reconfigure(config))
            .await
            .expect("command receiver closed");
    }

    pub fn frames(&self) -> broadcast::Receiver<Frame> {
        self.frame_channel
            .upgrade()
            .expect("frame sender closed")
            .subscribe()
    }
}

struct SamplingTask {
    sdr: RtlSdr,
    config: AntennaConfig,
    command_receiver: mpsc::Receiver<Command>,
    frame_sender: broadcast::Sender<Frame>,
    samples: Samples<Iq>,
    processing: Processing,
}

impl SamplingTask {
    async fn new(
        config: AntennaConfig,
        command_receiver: mpsc::Receiver<Command>,
        frame_sender: broadcast::Sender<Frame>,
    ) -> Result<Self, Error> {
        let processing = Processing::new(config.processing.clone());
        let sdr = open_sdr(&config.sdr)?;
        configure_sdr(&sdr, &config.sdr).await?;
        let samples = sdr.samples().await?;

        Ok(Self {
            sdr,
            config,
            command_receiver,
            frame_sender,
            samples,
            processing,
        })
    }

    async fn run(mut self) -> Result<(), Error> {
        loop {
            tokio::select! {
                command = self.command_receiver.recv() => {
                    let Some(command) = command else { break; };
                    self.handle_command(command).await?;
                }
                chunk = self.samples.try_next() => {
                    let chunk = chunk?.expect("samples: end of stream");
                    self.handle_chunk(chunk).await?;
                }
            };
        }

        Ok(())
    }

    async fn handle_command(&mut self, command: Command) -> Result<(), Error> {
        match command {
            Command::Reconfigure(config) => {
                if config.sdr != self.config.sdr {
                    tracing::info!("Reconfiguring SDR");
                    configure_sdr(&self.sdr, &config.sdr).await?;
                    self.config.sdr = config.sdr;
                }

                if config.processing != self.config.processing {
                    tracing::info!("Reconfiguring signal processing");
                    self.processing = Processing::new(config.processing.clone());
                    self.config.processing = config.processing;
                }
            }
        }

        Ok(())
    }

    async fn handle_chunk(&mut self, chunk: Chunk<Iq>) -> Result<(), Error> {
        for frame in self.processing.push_samples(chunk.as_ref()) {
            let stats = frame.stats();
            tracing::debug!(
                serial = frame.serial,
                min = stats.min,
                max = stats.max,
                average = stats.average,
                "Full frame"
            );

            // note: we ignore if the frame channel is closed. when the frame channel is
            // closed, the command channel is closed as well, which will cause the main loop
            // to exit.
            let _ = self.frame_sender.send(frame);
        }

        Ok(())
    }
}

#[derive(Debug)]
enum Command {
    Reconfigure(AntennaConfig),
}

fn open_sdr(config: &SdrConfig) -> Result<RtlSdr, Error> {
    let index = if let Some(find_serial) = &config.sdr_serial {
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

    Ok(RtlSdr::open(index)?)
}

async fn configure_sdr(sdr: &RtlSdr, config: &SdrConfig) -> Result<(), Error> {
    // configure SDR
    sdr.set_center_frequency(config.center_frequency).await?;
    sdr.set_sample_rate(config.sample_rate).await?;

    // select closest gain. rtlsdr-async can do this, but we want to log this
    let gain = {
        let available_gains = sdr.get_tuner_gains();
        let target_gain = (10.0 * config.tuner_gain) as i32;
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
    sdr.set_tuner_gain(gain).await?;

    if config.bias_tee {
        tracing::warn!("Bias-Tee is currently not supported");
    }
    //sdr.set_bias_tee(config.bias_tee).await?;

    Ok(())
}

#[derive(Debug)]
struct Processing {
    config: ProcessingConfig,
    sample_buffer: Vec<Complex<f32>>,
    freq_buffer: Vec<f32>,
    fft: Fft,
    average: Average,
    num_frames: usize,
}

impl Processing {
    pub fn new(config: ProcessingConfig) -> Self {
        let sample_buffer = Vec::with_capacity(config.window_size);
        let freq_buffer = Vec::with_capacity(config.window_size);
        let fft = Fft::new(config.window_size);
        let average = Average::new(config.window_size, config.average_size);

        Self {
            config,
            sample_buffer,
            freq_buffer,
            fft,
            average,
            num_frames: 0,
        }
    }

    pub fn push_samples(&mut self, samples: &[Iq]) -> Vec<Frame> {
        let mut averages = vec![];

        // convert samples to magnitude and buffer them
        for sample in samples {
            self.sample_buffer.push(Complex::from(*sample));

            if self.sample_buffer.len() == self.config.window_size {
                // fft samples
                self.fft.process(&mut self.sample_buffer);

                // take magnitude of spectrum
                assert!(self.freq_buffer.is_empty());
                for i in 0..self.config.window_size {
                    self.freq_buffer.push(self.sample_buffer[i].norm());
                }

                // average spectra
                if let Some(average) = self.average.push(&self.freq_buffer) {
                    averages.push(Frame {
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
                assert!(self.sample_buffer.len() < self.config.window_size)
            }
        }

        averages
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
