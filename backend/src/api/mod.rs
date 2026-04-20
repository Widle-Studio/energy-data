use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub mod data;
pub mod admin;

use crate::api::AppState;

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
