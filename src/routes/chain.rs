use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::models::chain::{ChainConfigResponse, FinalizePlanRequest, FinalizePlanResponse};
use crate::services::chain;

pub async fn get_chain_config(
    State(state): State<AppState>,
) -> Json<ChainConfigResponse> {
    Json(ChainConfigResponse {
        program_id: state.program_id.clone(),
        platform_config_pda: state.platform_config_pda.clone(),
        treasury_authority: state.treasury_authority.clone(),
        platform_fee_bps: state.platform_fee_bps,
        treasury_token_accounts: state.treasury_token_accounts.clone(),
    })
}

pub async fn build_finalize_plan(
    State(state): State<AppState>,
    Json(request): Json<FinalizePlanRequest>,
) -> Result<Json<FinalizePlanResponse>, (StatusCode, Json<Value>)> {
    let response = chain::build_finalize_plan(&state, request).map_err(|err| {
        let status = if err.to_string().contains("unsupported stake mint")
            || err.to_string().contains("cannot be empty")
        {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, Json(json!({"error": err.to_string()})))
    })?;

    Ok(Json(response))
}
