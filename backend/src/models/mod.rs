use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Country {
    pub id: Uuid,
    pub continent_id: Option<Uuid>,
    pub name: String,
    pub iso_alpha2: String,
    pub iso_alpha3: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Indicator {
    pub id: Uuid,
    pub source_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub unit: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct EnergyData {
    pub id: Uuid,
    pub country_id: Uuid,
    pub indicator_id: Uuid,
    pub year: i32,
    pub value: BigDecimal,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CommodityPrice {
    pub id: Uuid,
    pub symbol: String,
    pub price: BigDecimal,
    pub percent_change: BigDecimal,
    pub timestamp: DateTime<Utc>,
}
