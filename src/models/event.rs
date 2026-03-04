use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct EventLog {
    pub id: i64,
    pub signature: String,
    pub event_type: String,
    pub challenge_id: Option<String>,
    pub wallet_address: Option<String>,
    pub data: serde_json::Value,
    pub slot: i64,
    pub block_time: Option<DateTime<Utc>>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct EventQuery {
    pub event_type: Option<String>,
    pub limit: Option<i64>,
}