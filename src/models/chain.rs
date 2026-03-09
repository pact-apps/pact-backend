use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChainConfigResponse {
    pub program_id: String,
    pub platform_config_pda: String,
    pub treasury_authority: String,
    pub platform_fee_bps: u16,
    pub treasury_token_accounts: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct FinalizePlanRequest {
    pub challenge: String,
    pub vault: String,
    pub token_program: String,
    pub stake_mint: String,
    pub challenge_authority: Option<String>,
    pub participant_accounts: Vec<FinalizeParticipantAccount>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FinalizeParticipantAccount {
    pub participant_state: String,
    pub payout_token_account: String,
    pub commitment_profile: String,
}

#[derive(Debug, Serialize)]
pub struct FinalizePlanResponse {
    pub program_id: String,
    pub platform_config_pda: String,
    pub treasury_authority: String,
    pub treasury_token_account: String,
    pub platform_fee_bps: u16,
    pub challenge: String,
    pub vault: String,
    pub token_program: String,
    pub stake_mint: String,
    pub challenge_authority: Option<String>,
    pub remaining_accounts: Vec<String>,
    pub participant_triples: Vec<FinalizeParticipantAccount>,
    pub account_meta_order: Vec<AccountMetaPlan>,
}

#[derive(Debug, Serialize)]
pub struct AccountMetaPlan {
    pub role: String,
    pub pubkey: String,
}
