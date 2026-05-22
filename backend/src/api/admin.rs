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

use crate::{
    api::AppState,
    error::AppError,
    services::world_bank::{WorldBankService, WorldBankSync},
};

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
                    if token.len() == admin_token.len() {
                        token.as_bytes().ct_eq(admin_token.as_bytes()).into()
                    } else {
                        false
                    }
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
    handle_sync_worldbank(&service).await
}

async fn handle_sync_worldbank<S: WorldBankSync>(
    service: &S,
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
    use crate::services::world_bank::MockWorldBankSync;

    #[tokio::test]
    async fn test_handle_sync_worldbank_success() {
        let mut mock_service = MockWorldBankSync::new();
        mock_service
            .expect_sync_electricity_data()
            .times(1)
            .returning(|| Ok(42));

        let result = handle_sync_worldbank(&mock_service).await;

        assert!(result.is_ok());
        let json_value = result.unwrap().0;

        assert_eq!(json_value["status"], "success");
        assert_eq!(
            json_value["message"],
            "Successfully synchronized 42 records from World Bank"
        );
    }

    #[tokio::test]
    async fn test_handle_sync_worldbank_error() {
        let mut mock_service = MockWorldBankSync::new();
        mock_service
            .expect_sync_electricity_data()
            .times(1)
            .returning(|| Err(AppError::InternalServerError(anyhow::anyhow!("Sync failed"))));

        let result = handle_sync_worldbank(&mock_service).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InternalServerError(err) => {
                assert_eq!(err.to_string(), "Sync failed");
            }
            _ => panic!("Expected InternalServerError"),
        }
    }
}
