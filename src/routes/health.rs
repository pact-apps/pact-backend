use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::db;

pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let db_ok = db::ping(&state.db).await;

    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": if db_ok { "healthy" } else { "unhealthy" },
            "database": db_ok,
            "program_id": state.program_id,
            "solana_rpc": state.solana_rpc,
        })),
    )
}