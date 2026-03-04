use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChallengeMetadata {
    pub id: Uuid,
    pub challenge_id: String,
    pub challenge_pubkey: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertChallengeRequest {
    pub challenge_pubkey: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct ChallengeListResponse {
    pub challenges: Vec<ChallengeMetadata>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub created_by: Option<String>,
}