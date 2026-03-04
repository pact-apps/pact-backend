use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Proof {
    pub id: Uuid,
    pub challenge_id: String,
    pub wallet_address: String,
    pub proof_hash: String,
    pub file_path: String,
    pub file_name: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ProofResponse {
    pub challenge_id: String,
    pub wallet_address: String,
    pub proof_hash: String,
    pub file_name: String,
    pub content_type: String,
    pub file_size_bytes: i64,
    pub submitted_at: DateTime<Utc>,
}