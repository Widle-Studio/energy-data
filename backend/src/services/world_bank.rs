use bigdecimal::{BigDecimal, FromPrimitive};
use reqwest::Client;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::AppError;

const DEFAULT_WORLD_BANK_API_URL: &str = "https://api.worldbank.org/v2";
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
    api_base_url: String,
}

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
#[async_trait::async_trait]
pub trait WorldBankSync: Send + Sync {
    async fn sync_electricity_data(&self) -> Result<usize, AppError>;
}

impl WorldBankService {
    pub fn new(db: PgPool) -> Self {
        Self {
            client: Client::new(),
            db,
            api_base_url: DEFAULT_WORLD_BANK_API_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.api_base_url = url;
        self
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

    pub async fn ensure_countries<'a>(
        &self,
        countries: impl Iterator<Item = (&'a str, &'a str, &'a str)> + Clone,
    ) -> Result<std::collections::HashMap<String, Uuid>, AppError> {
        let countries_vec: Vec<_> = countries.collect();
        if countries_vec.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let mut mapping = std::collections::HashMap::new();

        // Postgres allows a maximum of 65535 parameters per query.
        // We have 3 parameters per country (name, iso_alpha2, iso_alpha3).
        let chunk_size = 65535 / 3;

        for chunk in countries_vec.chunks(chunk_size) {
            let mut query_builder: QueryBuilder<Postgres> =
                QueryBuilder::new("INSERT INTO countries (name, iso_alpha2, iso_alpha3) ");

            query_builder.push_values(chunk, |mut b, (name, iso2, iso3)| {
                b.push_bind(name).push_bind(iso2).push_bind(iso3);
            });

            query_builder.push(" ON CONFLICT (iso_alpha2) DO UPDATE SET name = EXCLUDED.name RETURNING iso_alpha2, id");

            let query = query_builder.build_query_as::<(String, Uuid)>();
            let results = query.fetch_all(&self.db).await?;

            for (iso2, id) in results {
                mapping.insert(iso2, id);
            }
        }

        Ok(mapping)
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

        let country_mapping = self.ensure_countries(countries.clone().into_iter()).await?;

        let futures = countries.into_iter().map(|(name, iso2, _iso3)| {
            let country_id = *country_mapping.get(iso2).unwrap();
            self.sync_single_country(name, iso2, country_id, indicator_id)
        });

        let results = futures::future::try_join_all(futures).await?;
        Ok(results.into_iter().sum())
    }

    async fn sync_single_country(
        &self,
        name: &str,
        iso2: &str,
        country_id: Uuid,
        indicator_id: Uuid,
    ) -> Result<usize, AppError> {
        let mut country_inserted_count = 0;

        let url = format!(
            "{}/country/{}/indicator/{}?format=json&per_page=100",
            self.api_base_url, iso2, ELECTRICITY_INDICATOR
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

        let tuples: Vec<_> = country_ids
            .into_iter()
            .zip(indicator_ids)
            .zip(years)
            .zip(values)
            .map(|(((c, i), y), v)| (c, i, y, v))
            .collect();

        if !tuples.is_empty() {
            let chunk_size = 65535 / 4;

            let futures = tuples.chunks(chunk_size).map(|chunk| {
                let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                    "INSERT INTO energy_data (country_id, indicator_id, year, value) ",
                );

                query_builder.push_values(
                    chunk,
                    |mut b, (country_id, indicator_id, year, value)| {
                        b.push_bind(*country_id)
                            .push_bind(*indicator_id)
                            .push_bind(*year)
                            .push_bind(value.clone());
                    },
                );

                query_builder.push(" ON CONFLICT (country_id, indicator_id, year) DO UPDATE SET value = EXCLUDED.value");

                let db = self.db.clone();
                async move {
                    query_builder.build().execute(&db).await
                }
            });

            let results = futures::future::join_all(futures).await;
            for result in results {
                match result {
                    Ok(res) => country_inserted_count += res.rows_affected() as usize,
                    Err(e) => tracing::error!("Failed to bulk insert records: {}", e),
                }
            }
        }

        Ok(country_inserted_count)
    }
}

#[async_trait::async_trait]
impl WorldBankSync for WorldBankService {
    async fn sync_electricity_data(&self) -> Result<usize, AppError> {
        self.sync_electricity_data().await
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
    use serde_json::json;
    use sqlx::PgPool;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[sqlx::test]
    async fn test_world_bank_service_new(pool: PgPool) {
        let service = WorldBankService::new(pool);

        // Verify the service can be cloned (required for Axum State)
        let _cloned = service.clone();
    }

    #[sqlx::test]
    async fn test_sync_electricity_data(pool: PgPool) {
        let db = pool.clone();

        let mock_server: MockServer = MockServer::start().await;

        let mock_response = json!([
            {
                "page": 1,
                "pages": 1,
                "per_page": 100,
                "total": 2,
                "sourceid": "2",
                "sourcename": "World Development Indicators",
                "lastupdated": "2024-03-28"
            },
            [
                {
                    "indicator": {
                        "id": "EG.USE.ELEC.KH.PC",
                        "value": "Electric power consumption (kWh per capita)"
                    },
                    "country": {
                        "id": "US",
                        "value": "United States"
                    },
                    "countryiso3code": "USA",
                    "date": "2014",
                    "value": 12993.9655794706,
                    "unit": "",
                    "obs_status": "",
                    "decimal": 0
                },
                {
                    "indicator": {
                        "id": "EG.USE.ELEC.KH.PC",
                        "value": "Electric power consumption (kWh per capita)"
                    },
                    "country": {
                        "id": "US",
                        "value": "United States"
                    },
                    "countryiso3code": "USA",
                    "date": "2013",
                    "value": 13004.0235687723,
                    "unit": "",
                    "obs_status": "",
                    "decimal": 0
                }
            ]
        ]);

        // Mock the World Bank API response for any country using a regex path
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/country/[A-Z]{2}/indicator/EG\.USE\.ELEC\.KH\.PC$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let service = WorldBankService::new(db.clone()).with_base_url(mock_server.uri());

        let result = service.sync_electricity_data().await;

        assert!(result.is_ok());

        // We mocked 4 countries (US, DE, CN, IN), each returning 2 records from the mock response.
        // Total expected insertions = 4 * 2 = 8
        let inserted_count = result.unwrap();
        assert_eq!(inserted_count, 8);

        // Verify data in the database
        let count: (i64,) = sqlx::query_as("SELECT count(*) FROM energy_data")
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(count.0, 8);
    }

    #[sqlx::test]
    async fn test_sync_electricity_data_api_error(pool: PgPool) {
        let db = pool.clone();
        let mock_server = MockServer::start().await;

        // Mock the World Bank API response to return a 500 error
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/country/[A-Z]{2}/indicator/EG\.USE\.ELEC\.KH\.PC$",
            ))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let service = WorldBankService::new(db.clone()).with_base_url(mock_server.uri());
        let result = service.sync_electricity_data().await;

        assert!(result.is_err());
        // result should be reqwest related error, or one wrapped in AppError
    }

    #[sqlx::test]
    async fn test_sync_electricity_data_invalid_json(pool: PgPool) {
        let db = pool.clone();
        let mock_server = MockServer::start().await;

        // Mock the World Bank API response to return invalid JSON
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/country/[A-Z]{2}/indicator/EG\.USE\.ELEC\.KH\.PC$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
            .mount(&mock_server)
            .await;

        let service = WorldBankService::new(db.clone()).with_base_url(mock_server.uri());
        let result = service.sync_electricity_data().await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Internal server error") || err_msg.contains("JSON"));
    }

    #[sqlx::test]
    async fn test_sync_electricity_data_empty_data(pool: PgPool) {
        let db = pool.clone();
        let mock_server = MockServer::start().await;

        // Mock the World Bank API response to return valid JSON but no data items
        let mock_response = json!([
            {
                "page": 1,
                "pages": 0,
                "per_page": 100,
                "total": 0,
                "sourceid": "2",
                "sourcename": "World Development Indicators",
                "lastupdated": "2024-03-28"
            },
            null
        ]);

        Mock::given(method("GET"))
            .and(path_regex(
                r"^/country/[A-Z]{2}/indicator/EG\.USE\.ELEC\.KH\.PC$",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&mock_server)
            .await;

        let service = WorldBankService::new(db.clone()).with_base_url(mock_server.uri());
        let result = service.sync_electricity_data().await;

        assert!(result.is_ok());
        let inserted_count = result.unwrap();
        assert_eq!(inserted_count, 0);

        // Verify data in the database
        let count: (i64,) = sqlx::query_as("SELECT count(*) FROM energy_data")
            .fetch_one(&db)
            .await
            .unwrap();

        assert_eq!(count.0, 0);
    }
}
