use std::{
    ops::Deref,
    path::Path,
};

use color_eyre::eyre::Error;
use sqlx::{
    SqlitePool,
    sqlite::SqliteConnectOptions,
};

#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();

        tracing::debug!(?path, "opening database");
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true),
        )
        .await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }
}

impl Deref for Database {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
