use std::sync::Arc;

use axum::{
    Router,
    extract::{
        State,
        WebSocketUpgrade,
        ws::{
            self,
            WebSocket,
        },
    },
    routing,
};
use color_eyre::eyre::Error;
use stolas_core::api::StatusEvent;
use tokio_util::sync::{
    CancellationToken,
    DropGuard,
};

use crate::station::Station;

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
            .route(
                "/status",
                routing::get(async move |State(api): State<Api>, ws: WebSocketUpgrade| {
                    ws.on_upgrade(async move |websocket: WebSocket| {
                        if let Err(error) = handle_status_websocket(api, websocket).await {
                            tracing::error!("{error}");
                        }
                    })
                }),
            )
            .with_state(self)
    }
}

async fn handle_status_websocket(api: Api, mut websocket: WebSocket) -> Result<(), Error> {
    let mut sensor_values = api.station.sensors().sensor_values();

    loop {
        tokio::select! {
            _ = api.shutdown.cancelled() => break,
            message = websocket.recv() => {
                // right now we don't expect any messages, but we can handle the socket closing, errors, and the close message.
                let Some(message) = message else { break; };
                let message = message?;
                match message {
                    ws::Message::Close(_) => break,
                    _ => {},
                }
            }
            result = sensor_values.changed() => {
                if result.is_err() {
                    // sensor values channel closed. this is not supposed to happen, but if it happens it should have reported an error already. we just exit then.
                    tracing::debug!("SensorValues channel closed. closing websocket");
                    break;
                }

                let message = {
                    // note: the sensor_values Ref can't be held across the await point, so we drop it before sending the message
                    let sensor_values = sensor_values.borrow_and_update();
                    ws::Message::Text(ws::Utf8Bytes::from(serde_json::to_string(&StatusEvent::Sensors(sensor_values.clone()))?))
                };
                websocket.send(message).await?;
            }
        }
    }

    Ok(())
}
