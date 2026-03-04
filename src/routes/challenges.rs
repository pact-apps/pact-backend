use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::{json, Value};

use crate::config::AppState;
use crate::models::challenge::*;

/// GET /api/challenges?limit=20&offset=0&created_by=WALLET
pub async fn list_challenges(
    State(state): State<AppState>,
    Query(params): Query<ChallengeQuery>,
) -> Result<Json<ChallengeListResponse>, (StatusCode, Json<Value>)> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let (challenges, total) = if let Some(ref creator) = params.created_by {
        let rows = sqlx::query_as::<_, ChallengeMetadata>(
            "SELECT * FROM challenge_metadata WHERE created_by = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(creator)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        })?;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM challenge_metadata WHERE created_by = $1"
        )
        .bind(creator)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        })?;

        (rows, count.0)
    } else {
        let rows = sqlx::query_as::<_, ChallengeMetadata>(
            "SELECT * FROM challenge_metadata ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        })?;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM challenge_metadata"
        )
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
        })?;

        (rows, count.0)
    };

    Ok(Json(ChallengeListResponse { challenges, total }))
}

/// GET /api/challenges/:challenge_id
pub async fn get_challenge(
    State(state): State<AppState>,
    Path(challenge_id): Path<String>,
) -> Result<Json<ChallengeMetadata>, (StatusCode, Json<Value>)> {
    let challenge = sqlx::query_as::<_, ChallengeMetadata>(
        "SELECT * FROM challenge_metadata WHERE challenge_id = $1"
    )
    .bind(&challenge_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    match challenge {
        Some(c) => Ok(Json(c)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Challenge not found"})),
        )),
    }
}

/// POST /api/challenges/:challenge_id/metadata
pub async fn upsert_metadata(
    State(state): State<AppState>,
    Path(challenge_id): Path<String>,
    Json(req): Json<UpsertChallengeRequest>,
) -> Result<Json<ChallengeMetadata>, (StatusCode, Json<Value>)> {
    let now = Utc::now();
    let description = req.description.unwrap_or_default();
    let tags = req.tags.unwrap_or_default();

    let result = sqlx::query_as::<_, ChallengeMetadata>(
        r#"
        INSERT INTO challenge_metadata (challenge_id, challenge_pubkey, title, description, tags, created_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        ON CONFLICT (challenge_id) DO UPDATE SET
            title = EXCLUDED.title,
            description = EXCLUDED.description,
            tags = EXCLUDED.tags,
            updated_at = $7
        RETURNING *
        "#
    )
    .bind(&challenge_id)
    .bind(&req.challenge_pubkey)
    .bind(&req.title)
    .bind(&description)
    .bind(&tags)
    .bind(&req.wallet_address)
    .bind(now)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    Ok(Json(result))
}