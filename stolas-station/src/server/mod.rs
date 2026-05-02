use std::sync::Arc;

use axum::Router;
use color_eyre::eyre::Error;
use tokio::net::TcpListener;
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::{
    server::api::Api,
    station::Station,
};

pub mod api;

pub struct Server {
    #[allow(unused)]
    drop_guard: Arc<DropGuard>,
}

impl Server {
    pub async fn new(server_address: &str, station: Station) -> Result<Self, Error> {
        tracing::info!("Starting server at http://{}", server_address);

        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();

        let router = Router::new().nest("/api", Api::new(station).into_router());

        let listener = TcpListener::bind(server_address).await?;

        let _join_handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    shutdown.cancelled().await;
                    tracing::info!("Shutting down server");
                })
                .await
        });

        Ok(Self {
            drop_guard: Arc::new(drop_guard),
        })
    }
}
