use std::{
    pin::Pin,
    task::{
        Context,
        Poll,
    },
};

use futures_util::{
    Stream,
    StreamExt,
};
use reqwest::Response;
use reqwest_websocket::{
    Upgrade,
    WebSocket,
};
use serde::de::DeserializeOwned;
use stolas_core::api::{
    ApiError,
    StatusEvent,
};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Websocket(#[from] reqwest_websocket::Error),
    #[error(transparent)]
    ApiError(#[from] ApiError),
}

#[derive(Clone, Debug)]
pub struct Client {
    client: reqwest::Client,
    api_url: Url,
}

impl Client {
    pub fn new(api_url: Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url,
        }
    }

    pub async fn status(&self) -> Result<Status, Error> {
        let websocket = self
            .client
            .get(self.api_url.join("status").unwrap())
            .upgrade()
            .send()
            .await?
            .into_websocket()
            .await?;

        Ok(Status { websocket })
    }

    pub async fn start(&self) -> Result<(), Error> {
        into_result(
            self.client
                .post(self.api_url.join("start").unwrap())
                .send()
                .await?,
        )
        .await
    }
}

async fn into_result<T>(response: Response) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    match response.error_for_status_ref() {
        Ok(_) => Ok(response.json().await?),
        Err(http_error) => {
            if let Ok(api_error) = response.json::<ApiError>().await {
                Err(Error::ApiError(api_error))
            }
            else {
                Err(http_error.into())
            }
        }
    }
}

#[derive(Debug)]
pub struct Status {
    websocket: WebSocket,
}

impl Stream for Status {
    type Item = Result<StatusEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.websocket.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error.into()))),
            Poll::Ready(Some(Ok(message))) => {
                match message.json() {
                    Ok(event) => Poll::Ready(Some(Ok(event))),
                    Err(error) => Poll::Ready(Some(Err(error.into()))),
                }
            }
        }
    }
}
