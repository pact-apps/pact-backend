use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::models::event::*;

/// GET /api/events/:challenge_id?event_type=ChallengeCreated&limit=50
pub async fn get_events(
    State(state): State<AppState>,
    Path(challenge_id): Path<String>,
    Query(params): Query<EventQuery>,
) -> Result<Json<Vec<EventLog>>, (StatusCode, Json<Value>)> {
    let limit = params.limit.unwrap_or(50).min(200);

    let events = if let Some(ref event_type) = params.event_type {
        sqlx::query_as::<_, EventLog>(
            "SELECT * FROM event_log WHERE challenge_id = $1 AND event_type = $2 ORDER BY indexed_at DESC LIMIT $3",
        )
        .bind(&challenge_id)
        .bind(event_type)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    } else {
        sqlx::query_as::<_, EventLog>(
            "SELECT * FROM event_log WHERE challenge_id = $1 ORDER BY indexed_at DESC LIMIT $2",
        )
        .bind(&challenge_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
    }
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    Ok(Json(events))
}