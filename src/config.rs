use std::collections::HashMap;

use anyhow::Context;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub solana_rpc: String,
    pub program_id: String,
    pub platform_fee_bps: u16,
    pub platform_config_pda: String,
    pub treasury_authority: String,
    pub treasury_token_accounts: HashMap<String, String>,
    pub jwt_secret: String,
    pub proof_storage_dir: String,
}

impl AppState {
    pub async fn new(db: PgPool, proof_storage_dir: String) -> anyhow::Result<Self> {
        let treasury_token_accounts = std::env::var("TREASURY_TOKEN_ACCOUNTS_JSON")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| serde_json::from_str::<HashMap<String, String>>(&value))
            .transpose()
            .context("TREASURY_TOKEN_ACCOUNTS_JSON must be valid JSON")?
            .unwrap_or_default();

        Ok(Self {
            db,
            solana_rpc: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".into()),
            program_id: std::env::var("PROGRAM_ID")
                .expect("PROGRAM_ID must be set"),
            platform_fee_bps: std::env::var("PLATFORM_FEE_BPS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(500),
            platform_config_pda: std::env::var("PLATFORM_CONFIG_PDA")
                .expect("PLATFORM_CONFIG_PDA must be set"),
            treasury_authority: std::env::var("TREASURY_AUTHORITY")
                .expect("TREASURY_AUTHORITY must be set"),
            treasury_token_accounts,
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            proof_storage_dir,
        })
    }
}
