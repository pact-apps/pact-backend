use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub challenge_type: String,
    pub proof_rule_config: Value,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ChallengeRuleConfig {
    pub challenge_id: String,
    pub required_checkins: Option<i32>,
    pub target_days: Option<i32>,
    pub grace_days: i32,
    pub rules_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ChallengeConfigResponse {
    pub metadata: ChallengeMetadata,
    pub rules: ChallengeRuleConfig,
}

#[derive(Debug, Deserialize)]
pub struct UpsertChallengeRequest {
    pub challenge_pubkey: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub wallet_address: Option<String>,
    pub challenge_type: Option<String>,
    pub proof_rule_config: Option<Value>,
    pub required_checkins: Option<i32>,
    pub target_days: Option<i32>,
    pub grace_days: Option<i32>,
    pub rules_json: Option<Value>,
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
