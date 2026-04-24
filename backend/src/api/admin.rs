use axum::{Json, Router, extract::State, routing::post};
use serde_json::json;

use crate::{api::AppState, error::AppError, services::world_bank::WorldBankService};

pub fn routes() -> Router<AppState> {
    Router::new().route("/sync/worldbank", post(sync_worldbank))
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
