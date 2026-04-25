use std::sync::Arc;

use chrono::Utc;
use color_eyre::eyre::Error;
use futures_util::TryStreamExt;
use num_complex::Complex;
use num_traits::Zero;
use rtlsdr_async::{
    Gain,
    Iq,
    RtlSdr,
};
use stolas_core::{
    Config,
    Frame,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub async fn handle_sampling(
    sdr: RtlSdr,
    config: Config,
    signal_sender: broadcast::Sender<Frame>,
    shutdown: CancellationToken,
) -> Result<(), Error> {
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

    //sdr.set_bias_tee(config.bias_tee).await?;

    // setup signal processing
    let mut processing = Processing::new(config.window_size, config.average_size);

    let mut samples = sdr.samples().await?;

    'outer: loop {
        let chunk = tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Shutdown signal. Stopping sampling.");
                break;
            },
            chunk = samples.try_next() => {
                chunk?.expect("samples: end of stream")
            }
        };

        for frame in processing.push_samples(chunk.as_ref()) {
            let stats = frame.stats();
            tracing::debug!(
                serial = frame.serial,
                min = stats.min,
                max = stats.max,
                average = stats.average,
                "Full frame"
            );

            if signal_sender.send(frame).is_err() {
                tracing::info!("Signal channel closed. Stopping sampling.");
                break 'outer;
            }
        }
    }

    Ok(())
}

#[derive(derive_more::Debug)]
struct Fft {
    #[debug(skip)]
    fft: Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    normalize: f32,
}

impl Fft {
    pub fn new(window_size: usize) -> Self {
        let mut fft_planner = rustfft::FftPlanner::new();
        let fft = fft_planner.plan_fft_forward(window_size);
        let scratch = vec![Complex::zero(); fft.get_inplace_scratch_len()];

        Self {
            fft,
            scratch,
            normalize: 1.0 / (window_size as f32),
        }
    }

    pub fn process(&mut self, buffer: &mut [Complex<f32>]) {
        self.fft.process_with_scratch(buffer, &mut self.scratch);
        for x in buffer {
            *x *= self.normalize;
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

    pub fn push(&mut self, values: &[f32]) -> Option<Arc<[f32]>> {
        assert_eq!(values.len(), self.sum.len());

        for (s, x) in self.sum.iter_mut().zip(values.iter()) {
            *s += *x;
        }

        self.count += 1;

        assert!(self.count <= self.size);
        if self.count == self.size {
            let output: Arc<[f32]> = self
                .sum
                .iter()
                .map(|sum| *sum / (self.size as f32))
                .collect();

            for s in &mut self.sum {
                *s = 0.0;
            }

            self.count = 0;

            Some(output)
        }
        else {
            None
        }
    }
}

#[derive(Debug)]
struct Processing {
    window_size: usize,
    sample_buffer: Vec<Complex<f32>>,
    freq_buffer: Vec<f32>,
    fft: Fft,
    average: Average,
    num_frames: usize,
}

impl Processing {
    pub fn new(window_size: usize, average_size: usize) -> Self {
        Self {
            window_size,
            sample_buffer: Vec::with_capacity(window_size),
            freq_buffer: Vec::with_capacity(window_size),
            fft: Fft::new(window_size),
            average: Average::new(window_size, average_size),
            num_frames: 0,
        }
    }

    pub fn push_samples(&mut self, samples: &[Iq]) -> Vec<Frame> {
        let mut averages = vec![];

        // convert samples to magnitude and buffer them
        for sample in samples {
            self.sample_buffer.push(Complex::from(*sample));

            if self.sample_buffer.len() == self.window_size {
                // fft samples
                self.fft.process(&mut self.sample_buffer);

                // take magnitude of spectrum
                assert!(self.freq_buffer.is_empty());
                for i in 0..self.window_size {
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
                assert!(self.sample_buffer.len() < self.window_size)
            }
        }

        averages
    }
}
