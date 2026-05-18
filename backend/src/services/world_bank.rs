use bigdecimal::{BigDecimal, FromPrimitive};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

const WORLD_BANK_API_URL: &str = "https://api.worldbank.org/v2";
const ELECTRICITY_INDICATOR: &str = "EG.USE.ELEC.KH.PC";

#[derive(Debug, Deserialize)]
pub struct WorldBankRecord {
    pub indicator: IndicatorMeta,
    pub country: CountryMeta,
    pub countryiso3code: String,
    pub date: String,
    pub value: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct IndicatorMeta {
    pub id: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct CountryMeta {
    pub id: String,
    pub value: String,
}

#[derive(Clone)]
pub struct WorldBankService {
    client: Client,
    db: PgPool,
}

impl WorldBankService {
    pub fn new(db: PgPool) -> Self {
        Self {
            client: Client::new(),
            db,
        }
    }

    pub async fn setup_metadata(&self) -> Result<(Uuid, Uuid), AppError> {
        let source_id = sqlx::query!(
            r#"
            INSERT INTO data_sources (name, url, description)
            VALUES ($1, $2, $3)
            ON CONFLICT (name) DO UPDATE SET url = EXCLUDED.url
            RETURNING id
            "#,
            "World Bank Open Data",
            "https://data.worldbank.org",
            "World Bank Open Data provides free and open access to global development data."
        )
        .fetch_one(&self.db)
        .await?
        .id;

        let indicator_id = sqlx::query!(
            r#"
            INSERT INTO indicators (source_id, name, code, unit, category)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
            source_id,
            "Electricity power consumption (kWh per capita)",
            ELECTRICITY_INDICATOR,
            "kWh per capita",
            "Energy"
        )
        .fetch_one(&self.db)
        .await?
        .id;

        Ok((source_id, indicator_id))
    }

    pub async fn ensure_country(
        &self,
        name: &str,
        iso2: &str,
        iso3: &str,
    ) -> Result<Uuid, AppError> {
        let country_id = sqlx::query!(
            r#"
            INSERT INTO countries (name, iso_alpha2, iso_alpha3)
            VALUES ($1, $2, $3)
            ON CONFLICT (iso_alpha2) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
            name,
            iso2,
            iso3
        )
        .fetch_one(&self.db)
        .await?
        .id;

        Ok(country_id)
    }

    pub async fn sync_electricity_data(&self) -> Result<usize, AppError> {
        let (_, indicator_id) = self.setup_metadata().await?;

        // Sample countries: USA, Germany, China, India
        let countries = vec![
            ("United States", "US", "USA"),
            ("Germany", "DE", "DEU"),
            ("China", "CN", "CHN"),
            ("India", "IN", "IND"),
        ];

        let futures = countries
            .into_iter()
            .map(|(name, iso2, iso3)| self.sync_single_country(name, iso2, iso3, indicator_id));

        let results = futures::future::join_all(futures).await;
        let mut inserted_count = 0;
        for result in results {
            inserted_count += result?;
        }

        Ok(inserted_count)
    }

    async fn sync_single_country(
        &self,
        name: &str,
        iso2: &str,
        iso3: &str,
        indicator_id: Uuid,
    ) -> Result<usize, AppError> {
        let country_id = self.ensure_country(name, iso2, iso3).await?;
        let mut country_inserted_count = 0;

        let url = format!(
            "{}/country/{}/indicator/{}?format=json&per_page=100",
            WORLD_BANK_API_URL, iso2, ELECTRICITY_INDICATOR
        );

        tracing::info!("Fetching data for {}: {}", name, url);

        let response = self.client.get(&url).send().await?.text().await?;

        // World bank returns an array where the second element is the array of data objects
        let parsed: serde_json::Value = serde_json::from_str(&response).map_err(|e| {
            tracing::error!("Failed to parse JSON: {}", e);
            AppError::InternalServerError(anyhow::anyhow!("Failed to parse JSON from World Bank"))
        })?;

        let Some(data_array) = parsed
            .as_array()
            .and_then(|arr| arr.get(1))
            .and_then(|v| v.as_array())
        else {
            tracing::warn!("No data found for {}", name);
            return Ok(country_inserted_count);
        };

        let mut country_ids = Vec::new();
        let mut indicator_ids = Vec::new();
        let mut years = Vec::new();
        let mut values = Vec::new();

        for item in data_array {
            let Some((year, value_bd)) = parse_world_bank_record(item) else {
                continue;
            };

            country_ids.push(country_id);
            indicator_ids.push(indicator_id);
            years.push(year);
            values.push(value_bd);
        }

        if !years.is_empty() {
            let chunk_size = 5000;

            for i in (0..years.len()).step_by(chunk_size) {
                let end = std::cmp::min(i + chunk_size, years.len());
                let result = sqlx::query!(
                    r#"
                        INSERT INTO energy_data (country_id, indicator_id, year, value)
                        SELECT * FROM UNNEST($1::uuid[], $2::uuid[], $3::int[], $4::numeric[])
                        ON CONFLICT (country_id, indicator_id, year)
                        DO UPDATE SET value = EXCLUDED.value
                        "#,
                    &country_ids[i..end],
                    &indicator_ids[i..end],
                    &years[i..end],
                    &values[i..end]
                )
                .execute(&self.db)
                .await;

                match result {
                    Ok(res) => country_inserted_count += res.rows_affected() as usize,
                    Err(e) => tracing::error!("Failed to bulk insert records: {}", e),
                }
            }
        }

        Ok(country_inserted_count)
    }
}

fn parse_world_bank_record(item: &serde_json::Value) -> Option<(i32, BigDecimal)> {
    let record = serde_json::from_value::<WorldBankRecord>(item.clone()).ok()?;
    let value = record.value?;
    let year = record.date.parse::<i32>().ok()?;
    let value_bd = BigDecimal::from_f64(value).unwrap_or_default();
    Some((year, value_bd))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    #[tokio::test]
    async fn test_world_bank_service_new() {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/testdb")
            .unwrap();

        let service = WorldBankService::new(db);

        // Verify the service can be cloned (required for Axum State)
        let _cloned = service.clone();
    }
}
