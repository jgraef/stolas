use std::io::SeekFrom;

use axum::{
    body::Body,
    extract::{
        Path,
        State,
    },
    http::{
        StatusCode,
        header,
    },
    response::{
        AppendHeaders,
        IntoResponse,
        Response,
    },
};
use stolas_core::api::InternalError;
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

use crate::server::api::{
    Api,
    error::ApiResponse,
};

pub async fn list_captures(State(api): State<Api>) -> impl IntoResponse {
    ApiResponse(api.station.captures().list().await)
}

pub async fn get_capture(State(api): State<Api>, Path(file_name): Path<String>) -> Response {
    let Ok(mut reader) = api.station.captures().read(&file_name).await
    else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // get file size
    // fixme: handle errors
    let file_size = reader.seek(SeekFrom::End(0)).await.unwrap();
    reader.seek(SeekFrom::Start(0)).await.unwrap();

    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let headers = AppendHeaders([
        (header::CONTENT_TYPE, "application/fits"),
        (
            header::CONTENT_DISPOSITION,
            &format!("attachment; filename=\"{file_name}\""),
        ),
        (header::CONTENT_LENGTH, &file_size.to_string()),
    ]);

    (headers, body).into_response()
}

pub async fn delete_capture(State(api): State<Api>, Path(file_name): Path<String>) -> StatusCode {
    if api.station.captures().delete(&file_name).await.is_err() {
        StatusCode::NOT_FOUND
    }
    else {
        StatusCode::OK
    }
}

pub async fn start_capture(State(api): State<Api>, Path(file_name): Path<String>) {
    todo!();
}

pub async fn stop_capture(State(api): State<Api>) {
    todo!();
}
