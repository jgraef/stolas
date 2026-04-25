use std::{
    fs::File,
    io::{
        BufReader,
        Read,
    },
    path::{
        Path,
        PathBuf,
    },
};

use byteorder::{
    BigEndian,
    ReadBytesExt,
};
use clap::{
    Parser,
    Subcommand,
};
use color_eyre::eyre::{
    Error,
    bail,
};
use stolas_core::{
    FileHeader,
    Frame,
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
}

fn read_file(path: impl AsRef<Path>) -> Result<(), Error> {
    let mut reader = BufReader::new(File::open(path)?);

    // read and verify file signature
    let mut signature = [0; 8];
    reader.read_exact(&mut signature)?;
    if &signature != b"STOLAS\x00\x01" {
        bail!("Invalid file signature");
    }

    // read header
    let header_size = reader.read_u32::<BigEndian>()?;
    let mut header_json = vec![0; header_size as usize];
    reader.read_exact(&mut header_json)?;
    let header: FileHeader = serde_json::from_slice(&header_json)?;
    tracing::debug!(?header, "Header");

    // read frames
    loop {
        // try to read frame and handle end-of-file gracefully
        let frame = match Frame::read(&mut reader) {
            Ok(frame) => {
                assert_eq!(frame.bins.len(), header.config.window_size);
                frame
            }
            Err(error) => {
                if error.kind() == std::io::ErrorKind::UnexpectedEof {
                    // end of file
                    break;
                }
                else {
                    return Err(error.into());
                }
            }
        };

        let stats = frame.stats();
        tracing::debug!(
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
