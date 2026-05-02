pub mod antenna;
pub mod captures;
pub mod error;
pub mod status;

use std::sync::Arc;

use axum::{
    Router,
    routing,
};
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::{
    server::api::{
        antenna::{
            get_antenna_config,
            get_antenna_stream,
            set_antenna_config,
        },
        captures::{
            delete_capture,
            get_capture,
            list_captures,
            start_capture,
            stop_capture,
        },
        status::get_status_stream,
    },
    station::Station,
};

#[derive(Clone, Debug)]
pub struct Api {
    station: Station,
    shutdown: CancellationToken,
    #[allow(unused)]
    drop_guard: Arc<DropGuard>,
}

impl Api {
    pub fn new(station: Station) -> Self {
        let shutdown = CancellationToken::new();
        let drop_guard = shutdown.clone().drop_guard();

        Self {
            station,
            shutdown,
            drop_guard: Arc::new(drop_guard),
        }
    }

    pub fn into_router(self) -> Router<()> {
        Router::new()
            .route("/status/ws", routing::get(get_status_stream))
            .route("/antenna/ws", routing::get(get_antenna_stream))
            .route("/antenna/config", routing::get(get_antenna_config))
            .route("/antenna/config", routing::post(set_antenna_config))
            .route("/captures", routing::get(list_captures))
            .route("/captures/{file_name}", routing::get(get_capture))
            .route("/captures/{file_name}", routing::delete(delete_capture))
            .route("/captures/start", routing::post(start_capture))
            .route("/captures/stop", routing::post(stop_capture))
            .with_state(self)
    }
}
