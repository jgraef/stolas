pub mod api;
pub mod network;
pub mod recording;
pub mod sample;
pub mod sensors;

use std::path::PathBuf;

use axum::{
    Router,
    response::{
        IntoResponse,
        Redirect,
    },
    routing,
};
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
use stolas_core::Config as SamplingConfig;
use tokio::{
    net::TcpListener,
    signal,
    sync::broadcast,
};
use tokio_util::sync::CancellationToken;
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::ServeDir,
};

use crate::{
    network::handle_network,
    recording::handle_recording,
    sample::handle_sampling,
    sensors::time::wait_for_time_sync,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    /*let project_dirs = ProjectDirs::from("org", "stolas", "station")
        .ok_or_else(|| eyre!("Could not determine project directories"))?;
    let data_path = project_dirs
        .state_dir()
        .ok_or_else(|| eyre!("Could not determine state directory"))?;
    let recordings_path = data_path.join("recordings");*/

    // todo
    let webui_path = "tmp/webui";
    tracing::debug!(?webui_path);

    let router = Router::new()
        // redirect / to /ui
        .route(
            "/",
            routing::any(async move || Redirect::temporary("/ui").into_response()),
        )
        // serve /ui from static files
        .nest_service(
            "/ui",
            ServeDir::new(webui_path).append_index_html_on_directories(true),
        )
        // serve /api/v1 with API
        .nest("/api/v1", api::router())
        // normalize paths
        .layer(NormalizePathLayer::trim_trailing_slash());

    tracing::info!("Listening at http://{}", args.listen_address);
    let listener = TcpListener::bind(&args.listen_address).await?;
    let shutdown = CancellationToken::new();
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { shutdown.cancelled().await })
        .await?;

    //old_main(args, recordings_path).await?;

    Ok(())
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long, env = "STATION_ADDRESS", default_value = "localhost:8080")]
    listen_address: String,

    #[clap(short, long, env = "STATION_DATA")]
    data_path: Option<PathBuf>,
}

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
