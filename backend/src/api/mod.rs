use axum::{
    Router,
    http::{Method, header},
    routing::get,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub mod admin;
pub mod data;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
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
        cors = cors.allow_origin(tower_http::cors::Any);
    }

    Router::new()
        .route("/api/v1/health", get(health_check))
        .nest("/api/v1/data", data::routes())
        .nest("/api/v1/admin", admin::routes())
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}
