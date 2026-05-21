use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use energtx_backend::{
    api::{AppState, create_router},
    config::Config,
    db,
};
use serde_json::Value;
use std::time::Instant;
use tower::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let pool = match db::setup_db_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            println!("Could not connect to database, skipping benchmark: {}", e);
            return Ok(());
        }
    };

    let cache = moka::future::Cache::builder()
        .time_to_live(std::time::Duration::from_secs(300))
        .max_capacity(100_000)
        .build();

    let app_state = AppState {
        db: pool,
        admin_token: config.admin_token,
        cache,
    };

    let app = create_router(app_state, config.allowed_origins);

    println!("Running baseline request (Cache Miss)...");

    let req1 = Request::builder()
        .uri("/api/v1/data?country=US&start_year=1990&end_year=2023")
        .body(Body::empty())
        .unwrap();

    let start = Instant::now();
    let response1 = app.clone().oneshot(req1).await?;
    let duration1 = start.elapsed();

    assert_eq!(response1.status(), StatusCode::OK);

    let body_bytes = to_bytes(response1.into_body(), usize::MAX).await?;
    let records: Vec<Value> = serde_json::from_slice(&body_bytes)?;

    println!("Baseline duration: {:?}", duration1);
    println!("Records returned: {}", records.len());

    println!("Running cached request (Cache Hit)...");

    let req2 = Request::builder()
        .uri("/api/v1/data?country=US&start_year=1990&end_year=2023")
        .body(Body::empty())
        .unwrap();

    let start = Instant::now();
    let response2 = app.clone().oneshot(req2).await?;
    let duration2 = start.elapsed();

    assert_eq!(response2.status(), StatusCode::OK);

    println!("Cached duration: {:?}", duration2);

    println!(
        "Performance improvement: {:.2}x",
        duration1.as_secs_f64() / duration2.as_secs_f64()
    );

    Ok(())
}
