use std::{
    fs::File,
    io::{
        BufWriter,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
};

use byteorder::{
    BigEndian,
    WriteBytesExt,
};
use chrono::Utc;
use color_eyre::eyre::Error;
use stolas_core::{
    Config,
    FileHeader,
    Frame,
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub async fn handle_recording(
    path: PathBuf,
    config: Config,
    mut signal_receiver: broadcast::Receiver<Frame>,
    shutdown: CancellationToken,
) -> Result<(), Error> {
    let mut writer = Writer::open(path, config)?;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Shutdown signal. Stopping recording.");
                break;
            },
            frame = signal_receiver.recv() => {
                match frame {
                    Ok(frame) => {
                        writer.push_frame(&frame)?;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(lag)) => {
                        tracing::warn!(lag, "Recording thread lagging");
                    }
                }
            }
        }
    }

    tracing::info!("Signal channel closed. Closing recording.");
    Ok(())
}

struct Writer {
    writer: BufWriter<File>,
}

impl Writer {
    pub fn open(path: PathBuf, config: Config) -> Result<Self, Error> {
        let writer = begin_file(&path, &config, 0)?;

        Ok(Self { writer })
    }

    pub fn push_frame(&mut self, frame: &Frame) -> Result<(), Error> {
        frame.write(&mut self.writer)?;
        Ok(())
    }
}

fn begin_file(path: &Path, config: &Config, serial: usize) -> Result<BufWriter<File>, Error> {
    let timestamp = Utc::now();

    std::fs::create_dir_all(path)?;

    let file_path = path.join(format!("{}.rec", timestamp.to_rfc3339()));
    let mut writer = BufWriter::new(File::create_new(&file_path)?);

    let header = FileHeader {
        timestamp,
        serial,
        config: config.clone(),
    };
    let header_json = serde_json::to_string(&header)?;

    writer.write_all(b"STOLAS\x00\x01")?;
    writer.write_u32::<BigEndian>(header_json.len().try_into().unwrap())?;
    writer.write_all(header_json.as_bytes())?;

    Ok(writer)
}
