use std::{
    path::{
        Path,
        PathBuf,
    },
    sync::Arc,
};

use color_eyre::eyre::{
    Error,
    bail,
};
use futures_util::TryStreamExt;
use parking_lot::RwLock;
use safe_path::scoped_join;
use stolas_core::{
    AntennaConfig,
    AntennaEvent,
    api::CaptureEntry,
    file::FileWriter,
};
use tokio::{
    fs::File,
    io::BufReader,
    sync::{
        broadcast,
        watch,
    },
    task::JoinHandle,
};
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::{
    database::Database,
    station::antenna::Antenna,
};

#[derive(Clone, Debug)]
pub struct Captures {
    database: Database,
    file_path: PathBuf,
    active: Arc<RwLock<Option<ActiveCapture>>>,
}

impl Captures {
    pub fn new(database: Database, file_path: impl AsRef<Path>) -> Result<Self, Error> {
        let file_path = file_path.as_ref();
        tracing::debug!(?file_path, "Captures path");

        if !file_path.exists() {
            std::fs::create_dir_all(file_path)?;
        }

        Ok(Self {
            database,
            file_path: file_path.to_owned(),
            active: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn list(&self) -> Result<Vec<CaptureEntry>, Error> {
        let active_file_name = self
            .active
            .read()
            .as_ref()
            .map(|active| active.file_name.clone());

        sqlx::query!("SELECT * from captures")
            .fetch(&*self.database)
            .map_ok(|row| {
                let active = active_file_name
                    .as_ref()
                    .is_some_and(|active_file_name| active_file_name == &row.file_name);
                CaptureEntry {
                    file_name: row.file_name,
                    active,
                }
            })
            .map_err(Error::from)
            .try_collect()
            .await
    }

    pub async fn read(&self, file_name: impl AsRef<str>) -> Result<BufReader<File>, Error> {
        let file_name = file_name.as_ref();

        if sqlx::query!(
            "SELECT file_name from captures WHERE file_name = ?",
            file_name
        )
        .fetch_one(&*self.database)
        .await
        .is_err()
        {
            bail!("Capture not found: {file_name}");
        }

        let file_path = scoped_join(&self.file_path, &file_name)?;

        Ok(BufReader::new(File::open(&file_path).await?))
    }

    pub async fn delete(&self, file_name: impl AsRef<str>) -> Result<(), Error> {
        let file_name = file_name.as_ref();

        if sqlx::query!("DELETE FROM captures WHERE file_name = ?", file_name)
            .execute(&*self.database)
            .await?
            .rows_affected()
            == 0
        {
            bail!("Capture not found: {file_name}");
        }

        let file_path = scoped_join(&self.file_path, &file_name)?;

        std::fs::remove_file(&file_path)?;

        Ok(())
    }

    pub fn start(&self, file_name: impl AsRef<str>, antenna: &Antenna) -> Result<(), Error> {
        let file_name = file_name.as_ref().to_owned();
        let file_path = self.file_path.join(&file_name);

        tracing::info!(file_name, "Starting capture");
        let writer = CaptureWriter::open(&file_path, antenna)?;

        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();

        let join_handle = tokio::spawn(async move {
            if let Err(error) = writer.run(shutdown).await {
                tracing::error!(%error, "capture writer failed");
            }
        });

        *self.active.write() = Some(ActiveCapture {
            file_name,
            drop_guard,
            join_handle,
        });

        Ok(())
    }

    pub fn stop(&self) {
        *self.active.write() = None;
    }
}

#[derive(Debug)]
struct ActiveCapture {
    file_name: String,
    #[allow(unused)]
    drop_guard: DropGuard,
    #[allow(unused)]
    join_handle: JoinHandle<()>,
}

#[derive(Debug)]
struct CaptureWriter {
    config: watch::Receiver<AntennaConfig>,
    events: broadcast::Receiver<AntennaEvent>,
    writer: FileWriter<std::io::BufWriter<std::fs::File>>,
}

impl CaptureWriter {
    fn open(path: impl AsRef<Path>, antenna: &Antenna) -> Result<Self, Error> {
        // fixme
        let _ = (path, antenna);

        /*let mut config = station.config().clone();
        let frames = station.antenna().frames();

        // initial config
        let initial_config = config.borrow_and_update().clone();

        // open writer. the initial antenna config will be written to the header
        let writer = FileWriter::open(
            path,
            &FileHeader {
                timestamp: Utc::now(),
                config: initial_config,
            },
        )?;

        Ok(Self {
            config,
            events,
            writer,
        })*/
        todo!();
    }

    async fn run(mut self, shutdown: CancellationToken) -> Result<(), Error> {
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = self.config.changed() => {
                    //let new_config = self.config.borrow_and_update().clone();
                }
                event = self.events.recv() => {
                    match event {
                        Ok(AntennaEvent::Frame(frame)) => {
                            self.writer.write_frame(&frame)?;
                        }
                        Ok(AntennaEvent::ConfigChanged(config)) => {
                            self.writer.write_config(&config)?;
                            // we ignore these events and use the config channel instead, as this is not affected by lag
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::debug!("frame channel closed. ending capture.");
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(lag)) => {
                            tracing::debug!(?lag, "frame channel lagging");
                            self.writer.write_dropped(lag)?;
                        }
                    }
                }
            }
        }

        // make sure the file is flushed
        self.writer.flush()?;

        Ok(())
    }
}
