use axum::{
    Json, Router,
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
    routing::post,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::{api::AppState, error::AppError, services::world_bank::WorldBankSync};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sync/worldbank", post(sync_worldbank))
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let admin_token = match &state.admin_token {
        Some(token) => token,
        None => {
            return Err(AppError::Unauthorized(
                "Admin token not configured on server".to_string(),
            ));
        }
    };

    let is_authorized = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "))
        .is_some_and(|token| {
            let provided_bytes = token.as_bytes();
            let expected_bytes = admin_token.as_bytes();

            if provided_bytes.len() != expected_bytes.len() {
                false
            } else {
                provided_bytes.ct_eq(expected_bytes).into()
            }
        });

    if !is_authorized {
        return Err(AppError::Unauthorized(
            "Invalid or missing admin token".to_string(),
        ));
    }

    Ok(next.run(req).await)
}

async fn sync_worldbank(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    handle_sync_worldbank(state.world_bank_service.as_ref()).await
}

async fn handle_sync_worldbank(
    service: &dyn WorldBankSync,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = service.sync_electricity_data().await?;

    Ok(Json(json!({
        "status": "success",
        "message": format!("Successfully synchronized {} records from World Bank", count)
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as AxumRequest, StatusCode},
        routing::{get, post},
    };
    use moka::future::Cache;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;
    use tower::util::ServiceExt;

    // Helper to create a test app with the auth middleware
    fn create_test_app(admin_token: Option<String>) -> Router {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/testdb")
            .unwrap();
        let cache = Cache::new(100);
        let mut mock_service = crate::services::world_bank::MockWorldBankSync::new();
        mock_service
            .expect_sync_electricity_data()
            .returning(|| Ok(0));
        let state = AppState {
            db,
            admin_token,
            cache,
            world_bank_service: Arc::new(mock_service),
        };

        Router::new()
            .route("/test", get(|| async { "Success" }))
            .route("/sync/worldbank", post(sync_worldbank))
            .route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_server_token() {
        let app = create_test_app(None);

        let request = AxumRequest::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer valid_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_header() {
        let app = create_test_app(Some("valid_token".to_string()));

        let request = AxumRequest::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_format() {
        let app = create_test_app(Some("valid_token".to_string()));

        let request = AxumRequest::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "valid_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_wrong_token() {
        let app = create_test_app(Some("valid_token".to_string()));

        let request = AxumRequest::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer wrong_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_wrong_token_length() {
        let app = create_test_app(Some("valid_token".to_string()));

        let request = AxumRequest::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer short")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_utf8_header() {
        let app = create_test_app(Some("valid_token".to_string()));

        // Create a header value with invalid UTF-8
        let invalid_utf8_header = axum::http::HeaderValue::from_bytes(b"Bearer \xFF\xFF").unwrap();

        let request = AxumRequest::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, invalid_utf8_header)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_correct_token() {
        let app = create_test_app(Some("valid_token".to_string()));

        let request = AxumRequest::builder()
            .uri("/test")
            .header(header::AUTHORIZATION, "Bearer valid_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_sync_worldbank_route_success() {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/testdb")
            .unwrap();
        let cache = Cache::new(100);
        let mut mock_service = crate::services::world_bank::MockWorldBankSync::new();
        mock_service
            .expect_sync_electricity_data()
            .times(1)
            .returning(|| Ok(42));

        let state = AppState {
            db,
            admin_token: Some("valid_token".to_string()),
            cache,
            world_bank_service: Arc::new(mock_service),
        };

        let app = Router::new()
            .route("/sync/worldbank", post(sync_worldbank))
            .route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        let request = AxumRequest::builder()
            .method("POST")
            .uri("/sync/worldbank")
            .header(header::AUTHORIZATION, "Bearer valid_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["status"], "success");
        assert_eq!(
            json["message"],
            "Successfully synchronized 42 records from World Bank"
        );
    }

    #[tokio::test]
    async fn test_sync_worldbank_route_failure() {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/testdb")
            .unwrap();
        let cache = Cache::new(100);
        let mut mock_service = crate::services::world_bank::MockWorldBankSync::new();
        mock_service
            .expect_sync_electricity_data()
            .times(1)
            .returning(|| {
                Err(crate::error::AppError::InternalServerError(
                    anyhow::anyhow!("Sync failed"),
                ))
            });

        let state = AppState {
            db,
            admin_token: Some("valid_token".to_string()),
            cache,
            world_bank_service: Arc::new(mock_service),
        };

        let app = Router::new()
            .route("/sync/worldbank", post(sync_worldbank))
            .route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .with_state(state);

        let request = AxumRequest::builder()
            .method("POST")
            .uri("/sync/worldbank")
            .header(header::AUTHORIZATION, "Bearer valid_token")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"], "Internal Server Error");
    }
}
