use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CommitmentScore {
    pub wallet_address: String,
    pub score: i32,
    pub challenges_joined: i32,
    pub challenges_completed: i32,
    pub challenges_failed: i32,
    pub challenges_disputed: i32,
    pub total_staked_lamports: i64,
    pub total_earned_lamports: i64,
    pub streak_current: i32,
    pub streak_best: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ScoreResponse {
    pub wallet_address: String,
    pub score: i32,
    pub rank: Option<i64>,
    pub challenges_completed: i32,
    pub challenges_failed: i32,
    pub streak_current: i32,
    pub streak_best: i32,
    pub total_staked_usdc: f64,
    pub total_earned_usdc: f64,
}

#[derive(Debug, Serialize)]
pub struct LeaderboardResponse {
    pub entries: Vec<LeaderboardEntry>,
    pub total: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LeaderboardEntry {
    pub wallet_address: String,
    pub score: i32,
    pub challenges_completed: i32,
    pub streak_best: i32,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}