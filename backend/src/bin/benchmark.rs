use energtx_backend::services::world_bank::WorldBankService;
use sqlx::postgres::PgPoolOptions;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/energy_data".to_string());

    let db = PgPoolOptions::new().connect(&database_url).await?;

    let service = WorldBankService::new(db);

    println!("Starting benchmark...");
    let start = Instant::now();

    match service.sync_electricity_data().await {
        Ok(count) => {
            let duration = start.elapsed();
            println!("Synchronized {} records in {:?}", count, duration);
        }
        Err(e) => {
            println!("Error during synchronization: {:?}", e);
        }
    }

    Ok(())
}
