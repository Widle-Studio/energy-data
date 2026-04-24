use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub mod admin;
pub mod data;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::permissive();

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
