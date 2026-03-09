use axum::{
    extract::{Extension, Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde_json::{json, Value};

use crate::config::AppState;
use crate::middleware::auth::Claims;
use crate::models::submission::{
    DisputeReviewResponse, FinalProofHashResponse, ParticipantSubmissionDetailResponse,
    SubmissionListQuery,
};
use crate::services::proofs;

struct UploadedPart {
    file_name: String,
    mime_type: String,
    bytes: Vec<u8>,
}

pub async fn upload_final_proof(
    State(state): State<AppState>,
    Path(challenge_id): Path<String>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let parsed = parse_multipart(multipart).await?;
    let final_result_claim = parsed
        .fields
        .get("final_result_claim")
        .map(|s| s.as_str());
    let (metadata, _) = proofs::get_challenge_with_rules(&state.db, &challenge_id)
        .await
        .map_err(internal_or_not_found)?;

    let submission = proofs::ensure_submission(
        &state.db,
        &challenge_id,
        &claims.sub,
        &metadata.challenge_type,
    )
    .await
    .map_err(internal_error)?;

    if metadata.challenge_type == "final_only" && parsed.files.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "At least one file is required for final_only challenges"})),
        ));
    }

    let mut saved_files = Vec::new();
    for file in &parsed.files {
        let saved = proofs::save_proof_file(
            &state.db,
            &state.proof_storage_dir,
            &challenge_id,
            &claims.sub,
            &metadata.challenge_type,
            "final",
            Some(submission.id),
            None,
            &file.file_name,
            &file.mime_type,
            &file.bytes,
        )
        .await
        .map_err(internal_error)?;
        saved_files.push(saved);
    }

    let submission = proofs::mark_submission_finalized(&state.db, submission.id, final_result_claim)
        .await
        .map_err(internal_error)?;
    let summary = proofs::generate_summary(&state.db, &challenge_id, &claims.sub)
        .await
        .map_err(internal_error)?;

    Ok(Json(json!({
        "submission": submission,
        "proof_files": saved_files,
        "summary": summary,
        "final_sha256": summary.final_sha256,
    })))
}

pub async fn upload_daily_checkin(
    State(state): State<AppState>,
    Path(challenge_id): Path<String>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let parsed = parse_multipart(multipart).await?;
    let (metadata, _) = proofs::get_challenge_with_rules(&state.db, &challenge_id)
        .await
        .map_err(internal_or_not_found)?;

    if metadata.challenge_type != "daily_checkin" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Challenge is not daily_checkin"})),
        ));
    }

    let submission = proofs::ensure_submission(
        &state.db,
        &challenge_id,
        &claims.sub,
        &metadata.challenge_type,
    )
    .await
    .map_err(internal_error)?;

    let checkin_date = parsed
        .fields
        .get("checkin_date")
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "checkin_date must use YYYY-MM-DD"})),
            )
        })?
        .unwrap_or_else(|| Utc::now().date_naive());
    let day_index = parsed
        .fields
        .get("day_index")
        .map(|value| value.parse::<i32>())
        .transpose()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "day_index must be an integer"})),
            )
        })?;
    let notes = parsed.fields.get("notes").cloned().unwrap_or_default();

    let checkin = proofs::upsert_daily_checkin(
        &state.db,
        &challenge_id,
        &claims.sub,
        submission.id,
        checkin_date,
        day_index,
        &notes,
    )
    .await
    .map_err(internal_error)?;

    let mut saved_files = Vec::new();
    for file in &parsed.files {
        let saved = proofs::save_proof_file(
            &state.db,
            &state.proof_storage_dir,
            &challenge_id,
            &claims.sub,
            &metadata.challenge_type,
            "checkin",
            Some(submission.id),
            Some(checkin.id),
            &file.file_name,
            &file.mime_type,
            &file.bytes,
        )
        .await
        .map_err(internal_error)?;
        saved_files.push(saved);
    }

    proofs::refresh_checkin_attachment_count(&state.db, checkin.id)
        .await
        .map_err(internal_error)?;
    let summary = proofs::generate_summary(&state.db, &challenge_id, &claims.sub)
        .await
        .map_err(internal_error)?;

    Ok(Json(json!({
        "checkin": checkin,
        "attachments": saved_files,
        "summary": summary,
        "final_sha256": summary.final_sha256,
    })))
}

pub async fn get_participant_submission(
    State(state): State<AppState>,
    Path((challenge_id, participant_wallet)): Path<(String, String)>,
) -> Result<Json<ParticipantSubmissionDetailResponse>, (StatusCode, Json<Value>)> {
    let submission = proofs::get_submission(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_or_not_found)?;
    let proof_files = proofs::list_proof_files(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_error)?;
    let daily_checkins = proofs::list_checkins(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_error)?;

    Ok(Json(ParticipantSubmissionDetailResponse {
        submission,
        proof_files,
        daily_checkins,
    }))
}

pub async fn list_submissions_for_challenge(
    State(state): State<AppState>,
    Path(challenge_id): Path<String>,
    Query(query): Query<SubmissionListQuery>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    let rows = sqlx::query_as::<_, crate::models::submission::ParticipantSubmission>(
        r#"
        SELECT * FROM participant_submissions
        WHERE challenge_id = $1
          AND ($2::VARCHAR IS NULL OR participant_wallet = $2)
        ORDER BY updated_at DESC
        "#,
    )
    .bind(&challenge_id)
    .bind(query.participant_wallet)
    .fetch_all(&state.db)
    .await
    .map_err(internal_error)?;

    Ok(Json(rows.into_iter().map(|row| json!(row)).collect()))
}

pub async fn generate_final_summary(
    State(state): State<AppState>,
    Path((challenge_id, participant_wallet)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let summary = proofs::generate_summary(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_or_not_found)?;

    Ok(Json(json!(summary)))
}

pub async fn get_final_summary(
    State(state): State<AppState>,
    Path((challenge_id, participant_wallet)): Path<(String, String)>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let summary = proofs::generate_summary(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_or_not_found)?;

    Ok(Json(json!(summary)))
}

pub async fn get_final_hash(
    State(state): State<AppState>,
    Path((challenge_id, participant_wallet)): Path<(String, String)>,
) -> Result<Json<FinalProofHashResponse>, (StatusCode, Json<Value>)> {
    let summary = proofs::generate_summary(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_or_not_found)?;

    Ok(Json(FinalProofHashResponse {
        challenge_id,
        participant_wallet,
        challenge_type: summary.challenge_type,
        final_sha256: summary.final_sha256,
        generated_at: summary.generated_at,
    }))
}

pub async fn get_dispute_review(
    State(state): State<AppState>,
    Path((challenge_id, participant_wallet)): Path<(String, String)>,
) -> Result<Json<DisputeReviewResponse>, (StatusCode, Json<Value>)> {
    let submission = proofs::get_submission(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_or_not_found)?;
    let summary = proofs::generate_summary(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_or_not_found)?;
    let proof_files = proofs::list_proof_files(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_error)?;
    let daily_checkins = proofs::list_checkins(&state.db, &challenge_id, &participant_wallet)
        .await
        .map_err(internal_error)?;

    Ok(Json(DisputeReviewResponse {
        challenge_id,
        participant_wallet,
        challenge_type: summary.challenge_type.clone(),
        submission,
        summary,
        proof_files,
        daily_checkins,
    }))
}

struct ParsedMultipart {
    fields: std::collections::HashMap<String, String>,
    files: Vec<UploadedPart>,
}

async fn parse_multipart(
    mut multipart: Multipart,
) -> Result<ParsedMultipart, (StatusCode, Json<Value>)> {
    let mut fields = std::collections::HashMap::new();
    let mut files = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "attachment" {
            files.push(UploadedPart {
                file_name: field
                    .file_name()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "proof.bin".to_string()),
                mime_type: field
                    .content_type()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                bytes: field
                    .bytes()
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": e.to_string()})),
                        )
                    })?
                    .to_vec(),
            });
        } else {
            let value = field
                .text()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;
            fields.insert(name, value);
        }
    }

    Ok(ParsedMultipart { fields, files })
}

fn internal_error<E: ToString>(err: E) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": err.to_string()})),
    )
}

fn internal_or_not_found<E: ToString>(err: E) -> (StatusCode, Json<Value>) {
    let status = if err.to_string().contains("not found") {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    (status, Json(json!({"error": err.to_string()})))
}
