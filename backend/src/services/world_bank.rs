use reqwest::Client;
use serde::{Deserialize, Serialize};
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

    pub async fn ensure_country(&self, name: &str, iso2: &str, iso3: &str) -> Result<Uuid, AppError> {
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

        let mut inserted_count = 0;

        for (name, iso2, iso3) in countries {
            let country_id = self.ensure_country(name, iso2, iso3).await?;

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

            if let Some(data_array) = parsed.as_array().and_then(|arr| arr.get(1)).and_then(|v| v.as_array()) {
                for item in data_array {
                    if let Ok(record) = serde_json::from_value::<WorldBankRecord>(item.clone()) {
                        if let (Some(value), Ok(year)) = (record.value, record.date.parse::<i32>()) {
                            let result = sqlx::query!(
                                r#"
                                INSERT INTO energy_data (country_id, indicator_id, year, value)
                                VALUES ($1, $2, $3, $4)
                                ON CONFLICT (country_id, indicator_id, year)
                                DO UPDATE SET value = EXCLUDED.value
                                "#,
                                country_id,
                                indicator_id,
                                year,
                                value
                            )
                            .execute(&self.db)
                            .await;

                            match result {
                                Ok(_) => inserted_count += 1,
                                Err(e) => tracing::error!("Failed to insert record: {}", e),
                            }
                        }
                    }
                }
            } else {
                tracing::warn!("No data found for {}", name);
            }
        }

        Ok(inserted_count)
    }
}
