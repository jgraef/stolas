use std::path::PathBuf;

use chrono::Utc;
use color_eyre::eyre::Error;
use stolas_core::{
    Config,
    Frame,
    file::{
        FileHeader,
        FileWriter,
    },
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub async fn handle_recording(
    path: PathBuf,
    config: Config,
    mut signal_receiver: broadcast::Receiver<Frame>,
    shutdown: CancellationToken,
) -> Result<(), Error> {
    let mut writer = FileWriter::open(
        path,
        &FileHeader {
            timestamp: Utc::now(),
            config,
        },
    )?;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("Shutdown signal. Stopping recording.");
                break;
            },
            frame = signal_receiver.recv() => {
                match frame {
                    Ok(frame) => {
                        writer.write_frame(&frame)?;
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
