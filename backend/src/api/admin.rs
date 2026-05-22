use axum::{
    Json, Router,
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
    routing::post,
};
use serde_json::json;
use subtle::ConstantTimeEq;

use crate::{api::AppState, error::AppError, services::world_bank::WorldBankService};

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

    let auth_header = req.headers().get(header::AUTHORIZATION);

    let is_authorized = match auth_header {
        Some(header_value) => {
            if let Ok(auth_str) = header_value.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    // Use constant-time comparison to prevent timing attacks
                    // Both strings must be the same length, otherwise it's immediately false
                    let is_len_match = token.len() == admin_token.len();
                    let compare_target = if is_len_match {
                        admin_token.as_bytes()
                    } else {
                        token.as_bytes()
                    };

                    let is_match: bool = token.as_bytes().ct_eq(compare_target).into();
                    is_len_match && is_match
                } else {
                    false
                }
            } else {
                false
            }
        }
        None => false,
    };

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
    let service = WorldBankService::new(state.db);

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
        routing::get,
    };
    use moka::future::Cache;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    // Helper to create a test app with the auth middleware
    fn create_test_app(admin_token: Option<String>) -> Router {
        let db = PgPoolOptions::new().connect_lazy("postgres://postgres:postgres@localhost:5432/testdb").unwrap();
        let cache = Cache::new(100);
        let state = AppState {
            db,
            admin_token,
            cache,
        };

        Router::new()
            .route(
                "/test",
                get(|| async { "Success" })
            )
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

        // Missing "Bearer " prefix
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
}
