use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use bigdecimal::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Serialize, Clone)]
pub struct DataResponse {
    country: String,
    indicator: String,
    year: i32,
    value: f64,
}

const DEFAULT_START_YEAR: i32 = 1990;
const DEFAULT_END_YEAR: i32 = 2025;

async fn get_data(
    State(state): State<AppState>,
    Query(params): Query<DataQuery>,
) -> Result<Json<Vec<DataResponse>>, AppError> {
    let country_filter = params.country.unwrap_or_default();
    let indicator_filter = params.indicator.unwrap_or_default();
    let start_year = params.start_year.unwrap_or(DEFAULT_START_YEAR);
    let end_year = params.end_year.unwrap_or(DEFAULT_END_YEAR);

    let cache_key = format!(
        "{}:{}:{}:{}",
        country_filter, indicator_filter, start_year, end_year
    );

    if let Some(cached_data) = state.cache.get(&cache_key).await {
        return Ok(Json((*cached_data).clone()));
    }

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

    let response: Vec<DataResponse> = records
        .into_iter()
        .map(|r| DataResponse {
            country: r.country_code,
            indicator: r.indicator_code,
            year: r.year,
            value: r.value.to_f64().unwrap_or(0.0), // sqlx returns BigDecimal/Numeric
        })
        .collect();

    state
        .cache
        .insert(cache_key, Arc::new(response.clone()))
        .await;

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use moka::future::Cache;
    use serde_json::Value;
    use sqlx::PgPool;
    use std::str::FromStr;
    use tower::ServiceExt;

    async fn setup_test_db(pool: &PgPool) {
        // Insert continent
        let continent_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        sqlx::query(
            "INSERT INTO continents (id, name, code) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
        )
        .bind(continent_id)
        .bind("Europe")
        .bind("EU")
        .execute(pool)
        .await
        .unwrap();

        // Insert country
        let country_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap();
        sqlx::query(
            "INSERT INTO countries (id, continent_id, name, iso_alpha2, iso_alpha3) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING"
        )
        .bind(country_id)
        .bind(continent_id)
        .bind("Germany")
        .bind("DE")
        .bind("DEU")
        .execute(pool)
        .await
        .unwrap();

        let country_id2 = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000012").unwrap();
        sqlx::query(
            "INSERT INTO countries (id, continent_id, name, iso_alpha2, iso_alpha3) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING"
        )
        .bind(country_id2)
        .bind(continent_id)
        .bind("France")
        .bind("FR")
        .bind("FRA")
        .execute(pool)
        .await
        .unwrap();

        // Insert data source
        let source_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000003").unwrap();
        sqlx::query(
            "INSERT INTO data_sources (id, name, url, description) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
        )
        .bind(source_id)
        .bind("Test Source")
        .bind("http://test.source")
        .bind("Test Description")
        .execute(pool)
        .await
        .unwrap();

        // Insert indicator
        let indicator_id = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000004").unwrap();
        sqlx::query(
            "INSERT INTO indicators (id, source_id, name, code, unit, category) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING"
        )
        .bind(indicator_id)
        .bind(source_id)
        .bind("Test Indicator")
        .bind("TEST.IND")
        .bind("Unit")
        .bind("Category")
        .execute(pool)
        .await
        .unwrap();

        let indicator_id2 = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000014").unwrap();
        sqlx::query(
            "INSERT INTO indicators (id, source_id, name, code, unit, category) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING"
        )
        .bind(indicator_id2)
        .bind(source_id)
        .bind("Another Test Indicator")
        .bind("OTHER.IND")
        .bind("Unit")
        .bind("Category")
        .execute(pool)
        .await
        .unwrap();

        // Insert energy data
        sqlx::query(
            "INSERT INTO energy_data (country_id, indicator_id, year, value) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
        )
        .bind(country_id)
        .bind(indicator_id)
        .bind(2020)
        .bind(bigdecimal::BigDecimal::from(100))
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO energy_data (country_id, indicator_id, year, value) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
        )
        .bind(country_id2)
        .bind(indicator_id)
        .bind(2021)
        .bind(bigdecimal::BigDecimal::from(200))
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO energy_data (country_id, indicator_id, year, value) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING"
        )
        .bind(country_id)
        .bind(indicator_id2)
        .bind(2022)
        .bind(bigdecimal::BigDecimal::from(300))
        .execute(pool)
        .await
        .unwrap();
    }

    fn create_test_app(db: PgPool) -> Router {
        let state = AppState {
            db,
            admin_token: None,
            cache: Cache::new(100),
            world_bank_service: Arc::new(crate::services::world_bank::MockWorldBankSync::new()),
        };

        Router::new().merge(routes()).with_state(state)
    }

    #[sqlx::test]
    async fn test_get_data_no_filters(pool: PgPool) {
        setup_test_db(&pool).await;
        let app = create_test_app(pool);

        let response = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();

        // We have 3 data points: DE 2020, FR 2021, and DE 2022
        assert_eq!(data.len(), 3);
    }

    #[sqlx::test]
    async fn test_get_data_with_country_filter(pool: PgPool) {
        setup_test_db(&pool).await;
        let app = create_test_app(pool);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?country=DE")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["country"], "DE");
        assert_eq!(data[0]["value"], 100.0);
        assert_eq!(data[1]["country"], "DE");
        assert_eq!(data[1]["value"], 300.0);

        let response_empty = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?country=US")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response_empty.status(), StatusCode::OK);
        let body_empty = axum::body::to_bytes(response_empty.into_body(), usize::MAX)
            .await
            .unwrap();
        let data_empty: Vec<Value> = serde_json::from_slice(&body_empty).unwrap();
        assert_eq!(data_empty.len(), 0);
    }

    #[sqlx::test]
    async fn test_get_data_with_year_filter(pool: PgPool) {
        setup_test_db(&pool).await;
        let app = create_test_app(pool);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?start_year=2021")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["country"], "FR");
        assert_eq!(data[0]["year"], 2021);
        assert_eq!(data[1]["country"], "DE");
        assert_eq!(data[1]["year"], 2022);
    }

    #[sqlx::test]
    async fn test_get_data_with_indicator_filter(pool: PgPool) {
        setup_test_db(&pool).await;
        let app = create_test_app(pool);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?indicator=OTHER.IND")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();

        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["indicator"], "OTHER.IND");
        assert_eq!(data[0]["country"], "DE");
        assert_eq!(data[0]["value"], 300.0);
    }

    #[sqlx::test]
    async fn test_get_data_with_end_year_filter(pool: PgPool) {
        setup_test_db(&pool).await;
        let app = create_test_app(pool);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?end_year=2020")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();

        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["country"], "DE");
        assert_eq!(data[0]["year"], 2020);
    }

    #[sqlx::test]
    async fn test_get_data_with_multiple_filters(pool: PgPool) {
        setup_test_db(&pool).await;
        let app = create_test_app(pool);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?country=DE&indicator=TEST.IND&start_year=2019&end_year=2021")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();

        assert_eq!(data.len(), 1);
        assert_eq!(data[0]["country"], "DE");
        assert_eq!(data[0]["indicator"], "TEST.IND");
        assert_eq!(data[0]["year"], 2020);
    }

    #[sqlx::test]
    async fn test_get_data_cache(pool: PgPool) {
        setup_test_db(&pool).await;

        let state = AppState {
            db: pool.clone(),
            admin_token: None,
            cache: Cache::new(100),
            world_bank_service: Arc::new(crate::services::world_bank::MockWorldBankSync::new()),
        };

        let app = Router::new().merge(routes()).with_state(state.clone());

        // First request populates the cache
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?country=DE")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response1.status(), StatusCode::OK);

        // Delete the data from db
        sqlx::query("DELETE FROM energy_data")
            .execute(&pool)
            .await
            .unwrap();

        // Second request should hit cache and still return data
        let response2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?country=DE")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response2.into_body(), usize::MAX)
            .await
            .unwrap();
        let data: Vec<Value> = serde_json::from_slice(&body).unwrap();

        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["country"], "DE");
    }
}
