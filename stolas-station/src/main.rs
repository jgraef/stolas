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
