use axum::{
    Router,
    http::{Method, header},
    middleware,
    routing::get,
};
use moka::future::Cache;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub mod admin;
pub mod data;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub admin_token: Option<String>,
    pub cache: Cache<String, Arc<Vec<data::DataResponse>>>,
    pub world_bank_service: std::sync::Arc<dyn crate::services::world_bank::WorldBankSync>,
}

pub fn create_router(state: AppState, allowed_origins: Vec<String>) -> Router {
    let mut cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    if !allowed_origins.is_empty() {
        let origins: Vec<header::HeaderValue> = allowed_origins
            .iter()
            .map(|s| s.parse().expect("Invalid origin"))
            .collect();
        cors = cors.allow_origin(origins);
    } else {
        // Explicitly deny all origins if allowed_origins is empty for security
        cors = cors.allow_origin(Vec::<header::HeaderValue>::new());
    }

    Router::new()
        .route("/api/v1/health", get(health_check))
        .nest("/api/v1/data", data::routes())
        .nest(
            "/api/v1/admin",
            admin::routes().route_layer(middleware::from_fn_with_state(
                state.clone(),
                admin::auth_middleware,
            )),
        )
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt; // for `oneshot`

    fn setup_state() -> AppState {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/testdb")
            .unwrap();
        let cache = Cache::new(100);
        use crate::services::world_bank::WorldBankService;
        let world_bank_service = std::sync::Arc::new(WorldBankService::new(db.clone()));

        AppState {
            db,
            admin_token: Some("test_token".to_string()),
            cache,
            world_bank_service,
        }
    }

    #[tokio::test]
    async fn test_cors_empty_origins() {
        let state = setup_state();
        let app = create_router(state, vec![]);

        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/health")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // When no origins are explicitly allowed (and no permissive defaults like Any),
        // tower-http cors will generally just return 200 OK without the Access-Control-Allow-Origin header
        // if the origin is not allowed, or reject it depending on exact configuration.
        // Let's just verify it processes the request.
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            !response
                .headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        );
    }

    #[tokio::test]
    async fn test_cors_specific_origin_allowed() {
        let state = setup_state();
        let allowed_origin = "https://allowed.example.com";
        let app = create_router(state, vec![allowed_origin.to_string()]);

        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/health")
            .header(header::ORIGIN, allowed_origin)
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            allowed_origin
        );
    }

    #[tokio::test]
    async fn test_cors_multiple_origins_allowed() {
        let state = setup_state();
        let allowed_origins = vec![
            "https://allowed1.example.com".to_string(),
            "https://allowed2.example.com".to_string(),
        ];
        let app = create_router(state, allowed_origins);

        // Test first allowed origin
        let request1 = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/health")
            .header(header::ORIGIN, "https://allowed1.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response1 = app.clone().oneshot(request1).await.unwrap();

        assert_eq!(response1.status(), StatusCode::OK);
        assert_eq!(
            response1
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "https://allowed1.example.com"
        );

        // Test second allowed origin
        let request2 = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/health")
            .header(header::ORIGIN, "https://allowed2.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response2 = app.oneshot(request2).await.unwrap();

        assert_eq!(response2.status(), StatusCode::OK);
        assert_eq!(
            response2
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "https://allowed2.example.com"
        );
    }

    #[tokio::test]
    async fn test_cors_specific_origin_rejected() {
        let state = setup_state();
        let allowed_origin = "https://allowed.example.com";
        let app = create_router(state, vec![allowed_origin.to_string()]);

        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/v1/health")
            .header(header::ORIGIN, "https://rejected.example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            !response
                .headers()
                .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        );
    }

    #[tokio::test]
    async fn test_routes_mounted_health() {
        let state = setup_state();
        let app = create_router(state, vec![]);

        let request = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_routes_mounted_data() {
        let state = setup_state();
        let app = create_router(state, vec![]);

        let request = Request::builder()
            .uri("/api/v1/data")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_routes_mounted_admin() {
        let state = setup_state();
        let app = create_router(state, vec![]);

        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/admin/sync/worldbank")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Without auth, it should hit the middleware and return 401
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
