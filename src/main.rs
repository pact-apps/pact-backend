use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod middleware;
mod models;
mod routes;
mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");
    let pool = sqlx::PgPool::connect(&database_url).await?;
    tracing::info!("Connected to database");

    let migration_sql = include_str!("../migrations/001_init.sql");
    for statement in migration_sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed)
                .execute(&pool)
                .await?;
        }
    }
    tracing::info!("Migrations applied");

    let proof_dir = std::env::var("PROOF_STORAGE_DIR")
        .unwrap_or_else(|_| "./proofs".into());
    tokio::fs::create_dir_all(&proof_dir).await?;

    let state = config::AppState::new(pool.clone(), proof_dir).await?;

    // Spawn Solana event listener as background task
    {
        let rpc_url = state.solana_rpc.clone();
        let program_id = state.program_id.clone();
        let listener_pool = pool;
        tokio::spawn(async move {
            services::solana_listener::poll_program_events(
                &listener_pool,
                &rpc_url,
                &program_id,
            )
            .await;
        });
        tracing::info!("Solana event listener spawned");
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Protected routes (require JWT auth)
    let protected = Router::new()
        .route("/api/chain/finalize-plan", post(routes::chain::build_finalize_plan))
        .route("/api/challenges/{challenge_id}/metadata", post(routes::challenges::upsert_metadata))
        .route("/api/challenges/{challenge_id}/proofs/final", post(routes::submissions::upload_final_proof))
        .route("/api/challenges/{challenge_id}/checkins", post(routes::submissions::upload_daily_checkin))
        .route("/api/proofs/upload", post(routes::proofs::upload_proof))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::require_auth,
        ));

    // Public routes
    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/api/chain/config", get(routes::chain::get_chain_config))
        .route("/api/challenges", get(routes::challenges::list_challenges))
        .route("/api/challenges/{challenge_id}", get(routes::challenges::get_challenge))
        .route("/api/challenges/{challenge_id}/config", get(routes::challenges::get_challenge_config))
        .route("/api/challenges/{challenge_id}/submissions", get(routes::submissions::list_submissions_for_challenge))
        .route("/api/challenges/{challenge_id}/submissions/{wallet}", get(routes::submissions::get_participant_submission))
        .route("/api/challenges/{challenge_id}/submissions/{wallet}/summary", get(routes::submissions::get_final_summary).post(routes::submissions::generate_final_summary))
        .route("/api/challenges/{challenge_id}/submissions/{wallet}/proof-hash", get(routes::submissions::get_final_hash))
        .route("/api/challenges/{challenge_id}/submissions/{wallet}/dispute-review", get(routes::submissions::get_dispute_review))
        .route("/api/proofs/{challenge_id}/{wallet}", get(routes::proofs::get_proof))
        .route("/api/events/{challenge_id}", get(routes::events::get_events))
        .route("/api/auth/nonce", get(routes::auth::get_nonce))
        .route("/api/auth/verify", post(routes::auth::verify_signature))
        .merge(protected)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Pact backend starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
