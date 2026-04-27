pub mod deploy;

use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre::Error;

use crate::deploy::deploy;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Deploy { host, path, run } => {
            deploy(&host, &path, run).await?;
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
    },
}
