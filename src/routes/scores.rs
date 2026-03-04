use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::models::score::*;

/// GET /api/scores/:wallet
pub async fn get_score(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> Result<Json<ScoreResponse>, (StatusCode, Json<Value>)> {
    // Ensure wallet has a score entry (upsert in two steps — sqlx doesn't support multi-statement)
    sqlx::query(
        "INSERT INTO commitment_scores (wallet_address) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(&wallet)
    .execute(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    let score = sqlx::query_as::<_, CommitmentScore>(
        "SELECT * FROM commitment_scores WHERE wallet_address = $1",
    )
    .bind(&wallet)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    // Get rank
    let rank: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) + 1 FROM commitment_scores WHERE score > $1",
    )
    .bind(score.score)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    Ok(Json(ScoreResponse {
        wallet_address: score.wallet_address,
        score: score.score,
        rank: Some(rank.0),
        challenges_completed: score.challenges_completed,
        challenges_failed: score.challenges_failed,
        streak_current: score.streak_current,
        streak_best: score.streak_best,
        // Convert lamports to USDC (6 decimals)
        total_staked_usdc: score.total_staked_lamports as f64 / 1_000_000.0,
        total_earned_usdc: score.total_earned_lamports as f64 / 1_000_000.0,
    }))
}

/// GET /api/scores?limit=20&offset=0
pub async fn leaderboard(
    State(state): State<AppState>,
    Query(params): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, (StatusCode, Json<Value>)> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let entries = sqlx::query_as::<_, LeaderboardEntry>(
        "SELECT wallet_address, score, challenges_completed, streak_best
         FROM commitment_scores
         ORDER BY score DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM commitment_scores")
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        })?;

    Ok(Json(LeaderboardResponse {
        entries,
        total: total.0,
    }))
}