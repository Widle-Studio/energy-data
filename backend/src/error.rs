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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use serde_json::Value;

    // Helper to extract JSON from response
    async fn get_response_json(response: axum::response::Response) -> Value {
        use axum::body::to_bytes;
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_database_error() {
        let err = AppError::DatabaseError(sqlx::Error::RowNotFound);
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let json = get_response_json(response).await;
        assert_eq!(json["error"], "Internal Database Error");
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let err = AppError::NotFound("Item not found".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let json = get_response_json(response).await;
        assert_eq!(json["error"], "Item not found");
    }

    #[tokio::test]
    async fn test_internal_server_error() {
        let err = AppError::InternalServerError(anyhow::anyhow!("Something went wrong"));
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let json = get_response_json(response).await;
        assert_eq!(json["error"], "Internal Server Error");
    }

    #[tokio::test]
    async fn test_request_error() {
        // Construct a dummy reqwest::Error
        let reqwest_err = reqwest::Client::new().get("").build().unwrap_err();
        let err = AppError::RequestError(reqwest_err);
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let json = get_response_json(response).await;
        assert_eq!(json["error"], "External API Error");
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let err = AppError::Unauthorized("Invalid token".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );

        let json = get_response_json(response).await;
        assert_eq!(json["error"], "Invalid token");
    }

    #[test]
    fn test_error_display() {
        let db_err = AppError::DatabaseError(sqlx::Error::RowNotFound);
        assert_eq!(db_err.to_string(), "Database error: no rows returned by a query that expected to return at least one row");

        let not_found_err = AppError::NotFound("User not found".to_string());
        assert_eq!(not_found_err.to_string(), "Not found: User not found");

        let internal_err = AppError::InternalServerError(anyhow::anyhow!("Something broke"));
        assert_eq!(internal_err.to_string(), "Internal server error");

        // Construct a dummy reqwest::Error
        let reqwest_underlying = reqwest::Client::new().get("").build().unwrap_err();
        let request_err = AppError::RequestError(reqwest_underlying);
        assert!(request_err.to_string().starts_with("Request error:"));

        let unauthorized_err = AppError::Unauthorized("Missing token".to_string());
        assert_eq!(unauthorized_err.to_string(), "Unauthorized: Missing token");
    }

    #[test]
    fn test_from_sqlx_error() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let app_err: AppError = sqlx_err.into();
        assert!(matches!(app_err, AppError::DatabaseError(_)));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("Some internal error");
        let app_err: AppError = anyhow_err.into();
        assert!(matches!(app_err, AppError::InternalServerError(_)));
    }

    #[test]
    fn test_from_reqwest_error() {
        let reqwest_err = reqwest::Client::new().get("").build().unwrap_err();
        let app_err: AppError = reqwest_err.into();
        assert!(matches!(app_err, AppError::RequestError(_)));
    }
}
