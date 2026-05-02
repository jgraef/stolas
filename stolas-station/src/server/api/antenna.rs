use axum::{
    Json,
    extract::{
        State,
        WebSocketUpgrade,
        ws::{
            self,
            WebSocket,
        },
    },
    response::IntoResponse,
};
use color_eyre::eyre::Error;
use stolas_core::{
    AntennaConfig,
    AntennaEvent,
    api::AntennaMessage,
};
use tokio::sync::broadcast;

use crate::server::api::Api;

pub async fn get_antenna_stream(State(api): State<Api>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(async move |websocket: WebSocket| {
        if let Err(error) = handle_antenna_stream(api, websocket).await {
            tracing::error!("{error}");
        }
    })
}

async fn handle_antenna_stream(api: Api, mut websocket: WebSocket) -> Result<(), Error> {
    let antenna = api.station.antenna();

    let mut config = antenna.config();
    websocket
        .send(ws::Message::Text(ws::Utf8Bytes::from(
            serde_json::to_string(&AntennaMessage::Config(config.clone()))?,
        )))
        .await?;

    let mut events = antenna.events();

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
            result = events.recv() => {
                match result {
                    Ok(AntennaEvent::Frame(frame)) => {
                        websocket.send(ws::Message::Text(ws::Utf8Bytes::from(serde_json::to_string(&AntennaMessage::Frame(frame))?))).await?;
                    }
                    Ok(AntennaEvent::ConfigChanged(new_config)) => {
                        config = new_config;
                        websocket.send(ws::Message::Text(ws::Utf8Bytes::from(serde_json::to_string(&AntennaMessage::Config(config.clone()))?))).await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::debug!("frame channel closed. closing websocket");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(lag)) => {
                        let new_config = antenna.config();
                        if new_config != config {
                            // config changed when we missed messages
                            config = new_config.clone();

                            websocket
                                .send(ws::Message::Text(ws::Utf8Bytes::from(
                                    serde_json::to_string(&AntennaMessage::Config(new_config))?,
                                )))
                                .await?;
                        }

                        websocket
                            .send(ws::Message::Text(ws::Utf8Bytes::from(
                                serde_json::to_string(&AntennaMessage::Lagged { lag })?,
                            )))
                            .await?;
                        tracing::debug!(?lag, "frame channel lagging");
                    }
                }



            }
        }
    }

    Ok(())
}

pub async fn get_antenna_config(State(api): State<Api>) -> Json<AntennaConfig> {
    Json(api.station.antenna().config())
}

pub async fn set_antenna_config(State(api): State<Api>, Json(config): Json<AntennaConfig>) {
    api.station.antenna().reconfigure(config).await;
}
