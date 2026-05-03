pub mod config;
pub mod database;
pub mod server;
pub mod station;
pub mod util;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::{
    Error,
    eyre,
};
use directories::ProjectDirs;
use tokio_util::sync::CancellationToken;

use crate::{
    database::Database,
    server::Server,
    station::Station,
    util::shutdown::cancel_on_ctrl_c_or_sigterm,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let project_dirs = ProjectDirs::from("org", "stolas", "stolas-station")
        .ok_or_else(|| eyre!("Could not determine project directories"))?;

    // read config
    let config_path = project_dirs.config_dir().join("config.toml");
    tracing::debug!(?config_path, "read config");
    let config = toml::from_str(&std::fs::read_to_string(&config_path)?)?;

    // create data-dir
    let data_dir = project_dirs.data_dir();
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)?;
    }

    // open database
    let database = Database::open(project_dirs.data_dir().join("db")).await?;

    //let data_path = project_dirs
    //    .state_dir()
    //    .ok_or_else(|| eyre!("Could not determine state directory"))?;
    //let recordings_path = data_path.join("recordings");

    // create the station sub-systems.

    let station = Station::new(config, database, data_dir).await?;

    // link Ctrl-C and SIGTERM to the shutdown CancellationToken
    let shutdown = CancellationToken::new();
    cancel_on_ctrl_c_or_sigterm(shutdown.clone());

    // start webserver
    let _server = if let Some(server_address) = args.server_address {
        Some(Server::new(&server_address, station.clone()).await?)
    }
    else {
        None
    };

    // wait for cancellation token
    shutdown.cancelled().await;

    tracing::debug!("shutting down");

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long, env = "STATION_ADDRESS")]
    server_address: Option<String>,

    #[clap(short, long, env = "STATION_DATA")]
    data_path: Option<PathBuf>,
}

/*
#[allow(dead_code)]
async fn old_main(
    config: SamplingConfig,
    listen_address: Option<String>,
    recordings_path: PathBuf,
) -> Result<(), Error> {
    // wait for clock to be synchronized via NTP
    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Received Ctrl-C. Quitting.");
            return Ok(());
        }
        result = wait_for_time_sync() => result?,
    }

    tracing::debug!("Connecting to RTL-SDR");
    let sdr = RtlSdr::open(0)?;

    let shutdown = CancellationToken::new();
    let (signal_sender, signal_receiver) = broadcast::channel(1024);

    let sampling_task = tokio::spawn({
        let config = config.clone();
        let shutdown = shutdown.clone();
        async move {
            if let Err(error) = handle_sampling(sdr, config, signal_sender, shutdown).await {
                tracing::error!("Sampling task failed: {error}");
            }
        }
    });

    let recorder_task = tokio::spawn({
        let path = recordings_path.clone();
        let config = config.clone();
        let signal_receiver = signal_receiver.resubscribe();
        let shutdown = shutdown.clone();
        async move {
            if let Err(error) = handle_recording(path, config, signal_receiver, shutdown).await {
                tracing::error!("Recorder task failed: {error}");
            }
        }
    });

    let network_task = if let Some(listen_address) = &listen_address {
        let listen_address = listen_address.clone();
        let shutdown = shutdown.clone();
        let config = config.clone();
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
 */
