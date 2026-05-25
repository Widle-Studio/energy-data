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
