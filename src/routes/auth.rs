use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::AppState;

/// Nonce — simple random string for wallet signature verification
#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub message: String,
}

/// GET /api/auth/nonce
/// Returns a nonce message that the wallet should sign
pub async fn get_nonce() -> Json<NonceResponse> {
    let nonce: u64 = rand::thread_rng().gen();
    let message = format!(
        "Sign this message to verify your wallet with Pact.\n\nNonce: {}",
        nonce
    );

    Json(NonceResponse {
        nonce: nonce.to_string(),
        message,
    })
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub wallet_address: String,  // base58 public key
    pub signature: String,       // base58 encoded signature
    pub message: String,         // the original message that was signed
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub wallet_address: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,   // wallet address
    exp: usize,    // expiration timestamp
    iat: usize,    // issued at
}

/// POST /api/auth/verify
/// Verify a wallet signature and return a JWT
pub async fn verify_signature(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<Value>)> {
    // Decode public key
    let pubkey_bytes = bs58::decode(&req.wallet_address)
        .into_vec()
        .map_err(|_| {
            (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid wallet address"})))
        })?;

    // Decode signature
    let sig_bytes = bs58::decode(&req.signature)
        .into_vec()
        .map_err(|_| {
            (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid signature"})))
        })?;

    // Verify ed25519 signature
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let verifying_key = VerifyingKey::from_bytes(
        &pubkey_bytes.try_into().map_err(|_| {
            (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid public key length"})))
        })?,
    )
    .map_err(|_| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid public key"})))
    })?;

    let signature = Signature::from_bytes(
        &sig_bytes.try_into().map_err(|_| {
            (StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid signature length"})))
        })?,
    );

    verifying_key
        .verify(req.message.as_bytes(), &signature)
        .map_err(|_| {
            (StatusCode::UNAUTHORIZED, Json(json!({"error": "Signature verification failed"})))
        })?;

    // Signature valid — issue JWT
    let now = Utc::now();
    let expires = now + Duration::hours(24);

    let claims = Claims {
        sub: req.wallet_address.clone(),
        exp: expires.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    Ok(Json(AuthResponse {
        token,
        wallet_address: req.wallet_address,
        expires_at: expires.to_rfc3339(),
    }))
}