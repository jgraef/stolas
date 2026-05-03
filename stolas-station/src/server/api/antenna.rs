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

    //let mut config = antenna.config().clone();
    let mut frames = antenna.frames();

    {
        // send current antenna config at startup and mark that value as seen in
        // the channel

        //let config = config.borrow_and_update().clone();
        //websocket
        //    .send(ws::Message::Text(ws::Utf8Bytes::from(
        //        serde_json::to_string(&AntennaMessage::Config(config))?,
        //    )))
        //    .await?;
    }

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
            /*_ = config.changed() => {
                let config = config.borrow().clone();
                websocket.send(ws::Message::Text(ws::Utf8Bytes::from(serde_json::to_string(&AntennaMessage::Config(config))?))).await?;
            }*/
            frame = frames.next_frame() => {
                websocket.send(ws::Message::Text(ws::Utf8Bytes::from(serde_json::to_string(&AntennaMessage::Frame(frame))?))).await?;
            }
        }
    }

    Ok(())
}

pub async fn get_antenna_config(State(api): State<Api>) -> Json<AntennaConfig> {
    //Json(api.station.antenna().config().borrow().clone())
    todo!();
}

pub async fn set_antenna_config(State(api): State<Api>, Json(config): Json<AntennaConfig>) {
    //api.station.antenna().reconfigure(config).await;
    todo!();
}
