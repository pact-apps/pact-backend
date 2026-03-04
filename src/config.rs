use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub solana_rpc: String,
    pub program_id: String,
    pub jwt_secret: String,
    pub proof_storage_dir: String,
}

impl AppState {
    pub async fn new(db: PgPool, proof_storage_dir: String) -> anyhow::Result<Self> {
        Ok(Self {
            db,
            solana_rpc: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".into()),
            program_id: std::env::var("PROGRAM_ID")
                .expect("PROGRAM_ID must be set"),
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            proof_storage_dir,
        })
    }
}