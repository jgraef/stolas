use std::path::{
    Path,
    PathBuf,
};

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre::Error;
use plotly::{
    Layout,
    Plot,
    Scatter3D,
    common::{
        ColorScale,
        ColorScalePalette,
        Marker,
        Mode,
        Title,
    },
    layout::{
        Axis,
        themes::BuiltinTheme,
    },
};
use stolas_core::{
    AntennaConfig,
    Frame,
    file::FileReader,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match &args.command {
        Command::Connect { address: _ } => todo!(),
        Command::Read { file } => {
            read_file(file)?;
        }
        Command::Plot { file } => {
            plot(file)?;
        }
    }

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Connect { address: String },
    Read { file: PathBuf },
    Plot { file: PathBuf },
}

fn read_file(path: impl AsRef<Path>) -> Result<(), Error> {
    let mut reader = FileReader::open(path)?;

    tracing::info!(header = ?reader.header(), "Header");

    while let Some(frame) = reader.read_frame()? {
        let stats = frame.stats();
        tracing::info!(
            time = %frame.timestamp,
            serial = frame.serial,
            min = stats.min,
            max = stats.max,
            average = stats.average,
            "Frame"
        );
    }

    Ok(())
}

fn plot(path: impl AsRef<Path>) -> Result<(), Error> {
    let mut reader = FileReader::open(path)?;

    tracing::info!(header = ?reader.header(), "Header");
    let config = reader.header().config.clone();

    /*let mut bin_index = closest_bin_for_frequency(&reader.header().config, 1420405751.0);
    if bin_index == 0 {
        tracing::warn!("Closest frequency bin is DC spike");
        // avoid DC spike
        bin_index += 1;
    }
    tracing::debug!(?bin_index);*/

    // read frames
    let mut frames = vec![];
    while let Some(frame) = reader.read_frame()? {
        // hard-coded fix for invalid timestamps in the test file
        if frame.serial < 5 {
            continue;
        }

        frames.push(frame);
    }

    // second average
    let second_average_size = 20;
    let mut frames = frames
        .chunks_exact(second_average_size)
        .map(|frames| {
            let mut bins = Vec::with_capacity(frames[0].bins.len());
            for bin_index in 0..config.processing.window_size {
                let amplitude = frames
                    .iter()
                    .map(|frame| frame.bins[bin_index])
                    .sum::<f32>()
                    / second_average_size as f32;
                bins.push(amplitude);
            }

            Frame {
                serial: frames[0].serial,
                timestamp: frames[0].timestamp,
                bins: bins.into(),
            }
        })
        .collect::<Vec<_>>();

    // normalize
    let mut ref_frame = vec![0.0; config.processing.window_size];
    for frame in &frames {
        for i in 0..config.processing.window_size {
            ref_frame[i] += frame.bins[i];
        }
    }
    for i in 0..config.processing.window_size {
        ref_frame[i] /= frames.len() as f32;
    }

    for frame in &mut frames {
        frame
            .bins
            .iter_mut()
            .zip(ref_frame.iter())
            .for_each(|(amplitude, reference)| *amplitude /= *reference);
    }

    // create plot data
    /*
        let mut timestamps = vec![];
        let mut powers = vec![vec![]; config.window_size];

        for frames in frames.iter() {
            timestamps.push(frames.timestamp.to_rfc3339());

            for bin_index in 0..config.window_size {
                let power_db = 20.0 * frames.bins[bin_index].log10();
                powers[bin_index].push(power_db);
            }
        }

        let mut plot = Plot::new();
        for (i, y) in powers.into_iter().enumerate() {
            if i == 0 {
                // dc spike
                continue;
            }
            let f = bin_center_frequency(&config, i);
            plot.add_trace(Scatter::new(timestamps.clone(), y).name(format!("#{i} - {f} MHz")));
        }

        plot.show();
    */

    let mut timestamps = vec![];
    let mut frequencies = vec![];
    let mut powers = vec![];

    for frames in &frames {
        for bin_index in 0..config.processing.window_size {
            if bin_index == 0 {
                // dc spike
                continue;
            }
            let power_db = 20.0 * frames.bins[bin_index].log10();

            timestamps.push(frames.timestamp.to_rfc3339());
            frequencies.push((bin_center_frequency(&config, bin_index) / 1000.0).round() / 1000.0);
            powers.push(power_db);
        }
    }

    let trace = Scatter3D::new(timestamps, frequencies, powers.clone())
        .mode(Mode::Markers)
        .marker(
            Marker::new()
                .color_scale(ColorScale::Palette(ColorScalePalette::Viridis))
                .color_array(powers.clone())
                .size(0),
        );

    let mut plot = Plot::new();
    plot.add_trace(trace);

    plot.set_layout(
        Layout::new()
            .title("21cm observation - Attempt #1")
            .template(BuiltinTheme::PlotlyWhite.build())
            .auto_size(true)
            .width(1600)
            .height(800)
            .x_axis(Axis::new().title(Title::from("Time")))
            .y_axis(Axis::new().title(Title::from("Frequency (MHz)")))
            .z_axis(Axis::new().title(Title::from("Power (dB)"))),
    );
    plot.show();

    Ok(())
}

#[allow(dead_code)]
fn closest_bin_for_frequency(config: &AntennaConfig, frequency: f32) -> usize {
    let bin_width = config.sdr.sample_rate as f32 / config.processing.window_size as f32;
    let mut bin_index =
        ((frequency - config.sdr.center_frequency as f32) / bin_width).round() as i32;
    if bin_index < 0 {
        bin_index += config.processing.window_size as i32;
    }
    bin_index as usize
}

fn bin_center_frequency(config: &AntennaConfig, i: usize) -> f32 {
    let bin_width = config.sdr.sample_rate as f32 / config.processing.window_size as f32;

    let offset = if i > config.processing.window_size / 2 {
        -(config.sdr.sample_rate as f32)
    }
    else {
        0.0
    };

    config.sdr.center_frequency as f32 + offset + i as f32 * bin_width
}
