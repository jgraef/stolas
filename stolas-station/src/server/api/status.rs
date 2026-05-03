use axum::{
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

use crate::server::api::Api;

pub async fn get_status_stream(State(api): State<Api>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(async move |websocket: WebSocket| {
        if let Err(error) = handle_status_stream(api, websocket).await {
            tracing::error!("{error}");
        }
    })
}

async fn handle_status_stream(api: Api, mut websocket: WebSocket) -> Result<(), Error> {
    //let mut sensor_values = api.station.sensors().sensor_values();

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
            /*result = sensor_values.changed() => {
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
            }*/
        }
    }

    Ok(())
}
