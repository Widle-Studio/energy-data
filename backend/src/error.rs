use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal server error")]
    InternalServerError(#[from] anyhow::Error),
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Database Error")
            }
            AppError::NotFound(ref msg) => (StatusCode::NOT_FOUND, msg.as_str()),
            AppError::InternalServerError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            AppError::RequestError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "External API Error"),
            AppError::Unauthorized(ref msg) => (StatusCode::UNAUTHORIZED, msg.as_str()),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}
