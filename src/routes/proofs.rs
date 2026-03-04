use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use sha2::{Digest, Sha256};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::models::proof::*;

/// POST /api/proofs/upload
/// Multipart form: challenge_id, wallet_address, file
pub async fn upload_proof(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ProofResponse>, (StatusCode, Json<Value>)> {
    let mut challenge_id: Option<String> = None;
    let mut wallet_address: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name = String::from("proof");
    let mut content_type = String::from("application/octet-stream");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "challenge_id" => {
                challenge_id = Some(
                    field.text().await.map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    })?,
                );
            }
            "wallet_address" => {
                wallet_address = Some(
                    field.text().await.map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    })?,
                );
            }
            "file" => {
                if let Some(fname) = field.file_name() {
                    file_name = fname.to_string();
                }
                if let Some(ct) = field.content_type() {
                    content_type = ct.to_string();
                }
                file_data = Some(
                    field.bytes().await.map_err(|e| {
                        (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()})))
                    })?.to_vec(),
                );
            }
            _ => {}
        }
    }

    let challenge_id = challenge_id.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "challenge_id required"})))
    })?;
    let wallet_address = wallet_address.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "wallet_address required"})))
    })?;
    let file_data = file_data.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, Json(json!({"error": "file required"})))
    })?;

    // Hash the file content (this is what goes on-chain as proof_hash)
    let mut hasher = Sha256::new();
    hasher.update(&file_data);
    let proof_hash = hex::encode(hasher.finalize());

    // Save file to disk
    let dir = format!("{}/{}", state.proof_storage_dir, challenge_id);
    tokio::fs::create_dir_all(&dir).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    let file_path = format!("{}/{}_{}", dir, wallet_address, file_name);
    tokio::fs::write(&file_path, &file_data).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    let file_size = file_data.len() as i64;

    // Upsert to database
    let proof = sqlx::query_as::<_, Proof>(
        r#"
        INSERT INTO proofs (challenge_id, wallet_address, proof_hash, file_path, file_name, content_type, file_size_bytes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (challenge_id, wallet_address) DO UPDATE SET
            proof_hash = EXCLUDED.proof_hash,
            file_path = EXCLUDED.file_path,
            file_name = EXCLUDED.file_name,
            content_type = EXCLUDED.content_type,
            file_size_bytes = EXCLUDED.file_size_bytes,
            submitted_at = NOW()
        RETURNING *
        "#,
    )
    .bind(&challenge_id)
    .bind(&wallet_address)
    .bind(&proof_hash)
    .bind(&file_path)
    .bind(&file_name)
    .bind(&content_type)
    .bind(file_size)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    Ok(Json(ProofResponse {
        challenge_id: proof.challenge_id,
        wallet_address: proof.wallet_address,
        proof_hash: proof.proof_hash,
        file_name: proof.file_name,
        content_type: proof.content_type,
        file_size_bytes: proof.file_size_bytes,
        submitted_at: proof.submitted_at,
    }))
}

/// GET /api/proofs/:challenge_id/:wallet
pub async fn get_proof(
    State(state): State<AppState>,
    Path((challenge_id, wallet)): Path<(String, String)>,
) -> Result<Json<ProofResponse>, (StatusCode, Json<Value>)> {
    let proof = sqlx::query_as::<_, Proof>(
        "SELECT * FROM proofs WHERE challenge_id = $1 AND wallet_address = $2",
    )
    .bind(&challenge_id)
    .bind(&wallet)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
    })?;

    match proof {
        Some(p) => Ok(Json(ProofResponse {
            challenge_id: p.challenge_id,
            wallet_address: p.wallet_address,
            proof_hash: p.proof_hash,
            file_name: p.file_name,
            content_type: p.content_type,
            file_size_bytes: p.file_size_bytes,
            submitted_at: p.submitted_at,
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Proof not found"})),
        )),
    }
}