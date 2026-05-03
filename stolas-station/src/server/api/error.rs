use axum::{
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use color_eyre::eyre::Error;
use serde::Serialize;
use stolas_core::api::InternalError;

#[derive(Clone, Debug)]
pub struct ApiResponse<S, E>(pub Result<S, E>);

impl<S, E> From<Result<S, E>> for ApiResponse<S, E> {
    fn from(value: Result<S, E>) -> Self {
        Self(value)
    }
}

impl<S, E> IntoResponse for ApiResponse<S, E>
where
    S: Serialize,
    E: IntoApiError,
{
    fn into_response(self) -> axum::response::Response {
        match self.0 {
            Ok(success) => (StatusCode::OK, Json(success)).into_response(),
            Err(error) => {
                let status_code = error.status_code();
                let error = error.into_api_error();
                (status_code, Json(error)).into_response()
            }
        }
    }
}

pub trait IntoApiError {
    type ApiError: Serialize;

    fn status_code(&self) -> StatusCode;
    fn into_api_error(self) -> Self::ApiError;
}

impl IntoApiError for Error {
    type ApiError = InternalError;

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn into_api_error(self) -> Self::ApiError {
        InternalError {
            message: format!("{self:#}"),
            backtrace: Some(format!("{self:?}")),
        }
    }
}
