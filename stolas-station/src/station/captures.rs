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
use futures_util::{
    Stream,
    StreamExt,
    TryStreamExt,
};
use parking_lot::RwLock;
use safe_path::scoped_join;
use stolas_core::{
    Frame,
    api::CaptureEntry,
};
use tokio::{
    fs::File,
    io::BufReader,
    task::JoinHandle,
};
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::database::Database;

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

    pub fn start(
        &self,
        file_name: impl AsRef<str>,
        frames: impl Stream<Item = Frame> + Send + Sync + Unpin + 'static,
    ) {
        let file_name = file_name.as_ref().to_owned();

        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();

        let join_handle = tokio::spawn({
            let file_name = file_name.clone();
            let file_path = self.file_path.join(&file_name);

            async move {
                tracing::info!(file_name, "Starting capture");

                if let Err(error) = write_capture(file_path, frames, shutdown).await {
                    tracing::error!(%error, "capture failed");
                }
                else {
                    tracing::info!(file_name, "Capture stopped");
                }

                // todo
            }
        });

        *self.active.write() = Some(ActiveCapture {
            file_name,
            drop_guard,
            join_handle,
        });
    }
}

#[derive(Debug)]
struct ActiveCapture {
    file_name: String,
    drop_guard: DropGuard,
    join_handle: JoinHandle<()>,
}

async fn write_capture(
    path: PathBuf,
    mut frames: impl Stream<Item = Frame> + Send + Sync + Unpin + 'static,
    shutdown: CancellationToken,
) -> Result<(), Error> {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            frame = frames.next() => {
                let Some(frame) = frame else {
                    tracing::warn!("frame stream closed");
                    break;
                };

                todo!();
                //capture_file.write_frame(&frame)?;
            }
        }
    }

    Ok(())
}
