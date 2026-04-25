pub mod network;
pub mod recording;
pub mod sample;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::Error;
use futures_util::{
    future::{
        self,
        Either,
    },
    pin_mut,
};
use rtlsdr_async::RtlSdr;
use stolas_core::Config;
use tokio::{
    signal,
    sync::broadcast,
};
use tokio_util::sync::CancellationToken;

use crate::{
    network::handle_network,
    recording::handle_recording,
    sample::handle_sampling,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    /*tracing::debug!("Loading config");
    let project_dirs = ProjectDirs::from("org", "stolas", "telescope")
        .ok_or_else(|| eyre!("Could not determine project directories"))?;
    let state_dir = project_dirs
        .state_dir()
        .ok_or_else(|| eyre!("Could not determine state directory"))?;
    let reader = BufReader::new(File::open(state_dir.join("config.json"))?);
    let config: Config = serde_json::from_reader(reader)?;*/

    tracing::debug!("Connecting to RTL-SDR");
    let sdr = RtlSdr::open(0)?;

    let shutdown = CancellationToken::new();
    let (signal_sender, signal_receiver) = broadcast::channel(1024);

    let sampling_task = tokio::spawn({
        let config = args.config.clone();
        let shutdown = shutdown.clone();
        async move {
            if let Err(error) = handle_sampling(sdr, config, signal_sender, shutdown).await {
                tracing::error!("Sampling task failed: {error}");
            }
        }
    });

    let recorder_task = tokio::spawn({
        let path = args.path.clone();
        let config = args.config.clone();
        let signal_receiver = signal_receiver.resubscribe();
        let shutdown = shutdown.clone();
        async move {
            if let Err(error) = handle_recording(path, config, signal_receiver, shutdown).await {
                tracing::error!("Recorder task failed: {error}");
            }
        }
    });

    let network_task = if let Some(listen_address) = &args.listen_address {
        let listen_address = listen_address.clone();
        let shutdown = shutdown.clone();
        let config = args.config.clone();
        Either::Left(tokio::spawn(async move {
            if let Err(error) =
                handle_network(listen_address, config, signal_receiver, shutdown).await
            {
                tracing::error!("Sampling task failed: {error}");
            }
        }))
    }
    else {
        Either::Right(async {
            // just wait for the shutdown signal
            shutdown.cancelled().await;
            Ok(())
        })
    };

    let tasks = future::join3(sampling_task, recorder_task, network_task);
    pin_mut!(tasks);

    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Received Ctrl-C. Quitting.");
            shutdown.cancel();
            tracing::info!("Shutdown signal. Waiting for all tasks to finish");
            let _ = tasks.await;
        }
        _ = &mut tasks => {}
    }

    Ok(())
}

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short, long)]
    listen_address: Option<String>,

    #[clap(short, long)]
    path: PathBuf,

    #[clap(flatten)]
    config: Config,
}
