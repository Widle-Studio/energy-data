use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use bigdecimal::ToPrimitive;
use serde::{Deserialize, Serialize};

use crate::{api::AppState, error::AppError};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(get_data))
}

#[derive(Deserialize)]
pub struct DataQuery {
    country: Option<String>,
    indicator: Option<String>,
    start_year: Option<i32>,
    end_year: Option<i32>,
}

#[derive(Serialize)]
pub struct DataResponse {
    country: String,
    indicator: String,
    year: i32,
    value: f64,
}

async fn get_data(
    State(state): State<AppState>,
    Query(params): Query<DataQuery>,
) -> Result<Json<Vec<DataResponse>>, AppError> {
    // Basic implementation querying the database
    let country_filter = params.country.unwrap_or_default();
    let indicator_filter = params.indicator.unwrap_or_default();
    let start_year = params.start_year.unwrap_or(1990);
    let end_year = params.end_year.unwrap_or(2025);

    let records = sqlx::query!(
        r#"
        SELECT
            c.iso_alpha2 as country_code,
            i.code as indicator_code,
            e.year,
            e.value
        FROM energy_data e
        JOIN countries c ON e.country_id = c.id
        JOIN indicators i ON e.indicator_id = i.id
        WHERE
            ($1 = '' OR c.iso_alpha2 = $1)
            AND ($2 = '' OR i.code = $2)
            AND e.year >= $3
            AND e.year <= $4
        ORDER BY e.year ASC
        "#,
        country_filter,
        indicator_filter,
        start_year,
        end_year
    )
    .fetch_all(&state.db)
    .await?;

    let response = records
        .into_iter()
        .map(|r| DataResponse {
            country: r.country_code,
            indicator: r.indicator_code,
            year: r.year,
            value: r.value.to_f64().unwrap_or(0.0), // sqlx returns BigDecimal/Numeric
        })
        .collect();

    Ok(Json(response))
}
