pub mod deploy;
pub mod dev;
pub mod util;
pub mod webui;

use std::path::PathBuf;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre::Error;

use crate::{
    deploy::deploy,
    dev::dev,
    webui::build_ui,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Deploy {
            host,
            path,
            run,
            no_build_ui,
        } => {
            if !no_build_ui {
                build_ui("./stolas-webui", "target/stolas-webui/dist", false, true).await?;
            }
            deploy(&host, &path, run).await?;
        }
        Command::BuildWebui {
            input,
            output,
            clean,
            release,
        } => {
            build_ui(&input, &output, clean, release).await?;
        }
        Command::Dev { watch } => {
            dev(watch).await?;
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
    /// Deploy stolas source to the telescope via rsync.
    Deploy {
        /// SSH hostname to deploy to
        #[clap(short = 'H', long, env = "DEPLOY_HOST")]
        host: String,

        /// Path on the SSH host to deploy to
        #[clap(short = 'p', long, env = "DEPLOY_PATH")]
        path: String,

        /// Run deployed code
        #[clap(short, long)]
        run: bool,

        /// Don't build the Web UI
        #[clap(short, long)]
        no_build_ui: bool,
    },
    /// Build the Web UI, ready for deployment.
    BuildWebui {
        #[clap(default_value = "stolas-webui")]
        input: PathBuf,

        #[clap(short, long, default_value = "target/stolas-webui/dist")]
        output: PathBuf,

        #[clap(short, long)]
        clean: bool,

        #[clap(short, long)]
        release: bool,
    },
    /// Build and run station server locally.
    ///
    /// This will also build the Web UI, watch for changes to it, and rebuilt it
    /// if necessary.
    Dev {
        #[clap(short, long)]
        watch: bool,
    },
}
