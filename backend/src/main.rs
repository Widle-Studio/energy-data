pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod services;
pub mod utils;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "energtx_backend=debug,axum=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env()?;

    tracing::info!("Connecting to database...");
    let pool = db::setup_db_pool(&config.database_url).await?;

    // Run migrations
    tracing::info!("Running migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app_state = api::AppState {
        db: pool,
        admin_token: config.admin_token,
    };
    let app = api::create_router(app_state, config.allowed_origins);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
