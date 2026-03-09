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
            "platform_config_pda": state.platform_config_pda,
            "treasury_authority": state.treasury_authority,
            "platform_fee_bps": state.platform_fee_bps,
            "solana_rpc": state.solana_rpc,
        })),
    )
}
