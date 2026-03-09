use std::path::Path;

use anyhow::{anyhow, bail, Context};
use chrono::NaiveDate;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::challenge::{ChallengeMetadata, ChallengeRuleConfig};
use crate::models::submission::{
    DailyCheckin, FinalProofSummary, ParticipantSubmission, ProofFile,
};

pub async fn get_challenge_with_rules(
    pool: &PgPool,
    challenge_id: &str,
) -> anyhow::Result<(ChallengeMetadata, ChallengeRuleConfig)> {
    let metadata = sqlx::query_as::<_, ChallengeMetadata>(
        "SELECT * FROM challenge_metadata WHERE challenge_id = $1",
    )
    .bind(challenge_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("Challenge not found"))?;

    let rules = sqlx::query_as::<_, ChallengeRuleConfig>(
        "SELECT * FROM challenge_rules WHERE challenge_id = $1",
    )
    .bind(challenge_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(ChallengeRuleConfig {
        challenge_id: challenge_id.to_string(),
        required_checkins: None,
        target_days: None,
        grace_days: 0,
        rules_json: json!({}),
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    });

    Ok((metadata, rules))
}

pub async fn ensure_submission(
    pool: &PgPool,
    challenge_id: &str,
    participant_wallet: &str,
    challenge_type: &str,
) -> anyhow::Result<ParticipantSubmission> {
    let submission = sqlx::query_as::<_, ParticipantSubmission>(
        r#"
        INSERT INTO participant_submissions (challenge_id, participant_wallet, challenge_type)
        VALUES ($1, $2, $3)
        ON CONFLICT (challenge_id, participant_wallet) DO UPDATE SET
            challenge_type = EXCLUDED.challenge_type,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .bind(challenge_type)
    .fetch_one(pool)
    .await?;

    Ok(submission)
}

pub async fn save_proof_file(
    pool: &PgPool,
    storage_root: &str,
    challenge_id: &str,
    participant_wallet: &str,
    challenge_type: &str,
    proof_kind: &str,
    submission_id: Option<Uuid>,
    checkin_id: Option<Uuid>,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> anyhow::Result<ProofFile> {
    let sha256 = hex::encode(Sha256::digest(bytes));
    let kind_dir = match proof_kind {
        "final" => "final",
        "checkin" => "checkins",
        other => other,
    };
    let dir = Path::new(storage_root)
        .join(challenge_id)
        .join(participant_wallet)
        .join(kind_dir);
    tokio::fs::create_dir_all(&dir).await?;

    let safe_name = if file_name.is_empty() {
        format!("{}.bin", Uuid::new_v4())
    } else {
        sanitize_file_name(file_name)
    };
    let storage_path = dir.join(format!("{}_{}", Uuid::new_v4(), safe_name));
    tokio::fs::write(&storage_path, bytes).await?;

    let storage_path_str = storage_path.to_string_lossy().to_string();
    let file_url = storage_path_str.clone();
    let file_size = i64::try_from(bytes.len()).context("file too large")?;

    let proof_file = sqlx::query_as::<_, ProofFile>(
        r#"
        INSERT INTO proof_files (
            challenge_id, participant_wallet, submission_id, checkin_id, challenge_type,
            proof_kind, storage_path, file_url, file_name, mime_type, sha256, size_bytes
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        RETURNING *
        "#,
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .bind(submission_id)
    .bind(checkin_id)
    .bind(challenge_type)
    .bind(proof_kind)
    .bind(&storage_path_str)
    .bind(&file_url)
    .bind(safe_name)
    .bind(mime_type)
    .bind(&sha256)
    .bind(file_size)
    .fetch_one(pool)
    .await?;

    Ok(proof_file)
}

pub async fn upsert_daily_checkin(
    pool: &PgPool,
    challenge_id: &str,
    participant_wallet: &str,
    submission_id: Uuid,
    checkin_date: NaiveDate,
    day_index: Option<i32>,
    notes: &str,
) -> anyhow::Result<DailyCheckin> {
    let checkin = sqlx::query_as::<_, DailyCheckin>(
        r#"
        INSERT INTO daily_checkins (
            challenge_id, participant_wallet, submission_id, checkin_date, day_index, notes, attachment_count
        )
        VALUES ($1, $2, $3, $4, $5, $6, 0)
        ON CONFLICT (challenge_id, participant_wallet, checkin_date) DO UPDATE SET
            day_index = EXCLUDED.day_index,
            notes = EXCLUDED.notes,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .bind(submission_id)
    .bind(checkin_date)
    .bind(day_index)
    .bind(notes)
    .fetch_one(pool)
    .await?;

    Ok(checkin)
}

pub async fn refresh_checkin_attachment_count(
    pool: &PgPool,
    checkin_id: Uuid,
) -> anyhow::Result<()> {
    let attachment_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM proof_files WHERE checkin_id = $1",
    )
    .bind(checkin_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "UPDATE daily_checkins SET attachment_count = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(attachment_count.0 as i32)
    .bind(checkin_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_submission_finalized(
    pool: &PgPool,
    submission_id: Uuid,
    final_result_claim: Option<&str>,
) -> anyhow::Result<ParticipantSubmission> {
    let submission = sqlx::query_as::<_, ParticipantSubmission>(
        r#"
        UPDATE participant_submissions
        SET status = 'final_proof_uploaded',
            final_result_claim = COALESCE($2, final_result_claim),
            finalized_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(submission_id)
    .bind(final_result_claim)
    .fetch_one(pool)
    .await?;

    Ok(submission)
}

pub async fn get_submission(
    pool: &PgPool,
    challenge_id: &str,
    participant_wallet: &str,
) -> anyhow::Result<ParticipantSubmission> {
    let submission = sqlx::query_as::<_, ParticipantSubmission>(
        "SELECT * FROM participant_submissions WHERE challenge_id = $1 AND participant_wallet = $2",
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("Submission not found"))?;

    Ok(submission)
}

pub async fn list_proof_files(
    pool: &PgPool,
    challenge_id: &str,
    participant_wallet: &str,
) -> anyhow::Result<Vec<ProofFile>> {
    let files = sqlx::query_as::<_, ProofFile>(
        r#"
        SELECT * FROM proof_files
        WHERE challenge_id = $1 AND participant_wallet = $2
        ORDER BY uploaded_at ASC
        "#,
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .fetch_all(pool)
    .await?;

    Ok(files)
}

pub async fn list_checkins(
    pool: &PgPool,
    challenge_id: &str,
    participant_wallet: &str,
) -> anyhow::Result<Vec<DailyCheckin>> {
    let checkins = sqlx::query_as::<_, DailyCheckin>(
        r#"
        SELECT * FROM daily_checkins
        WHERE challenge_id = $1 AND participant_wallet = $2
        ORDER BY checkin_date ASC, created_at ASC
        "#,
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .fetch_all(pool)
    .await?;

    Ok(checkins)
}

pub async fn generate_summary(
    pool: &PgPool,
    challenge_id: &str,
    participant_wallet: &str,
) -> anyhow::Result<FinalProofSummary> {
    let (metadata, rules) = get_challenge_with_rules(pool, challenge_id).await?;
    let submission = get_submission(pool, challenge_id, participant_wallet).await?;
    let proof_files = list_proof_files(pool, challenge_id, participant_wallet).await?;
    let daily_checkins = list_checkins(pool, challenge_id, participant_wallet).await?;

    let evaluation = evaluate_submission(&metadata.challenge_type, &rules, &proof_files, &daily_checkins);

    let proof_files_json: Vec<Value> = proof_files
        .iter()
        .map(|file| {
            json!({
                "id": file.id,
                "proof_kind": file.proof_kind,
                "file_name": file.file_name,
                "mime_type": file.mime_type,
                "sha256": file.sha256,
                "size_bytes": file.size_bytes,
                "uploaded_at": file.uploaded_at,
                "checkin_id": file.checkin_id,
                "file_url": file.file_url,
            })
        })
        .collect();

    let daily_checkins_json: Vec<Value> = daily_checkins
        .iter()
        .map(|checkin| {
            let attachment_hashes: Vec<String> = proof_files
                .iter()
                .filter(|file| file.checkin_id == Some(checkin.id))
                .map(|file| file.sha256.clone())
                .collect();

            json!({
                "id": checkin.id,
                "checkin_date": checkin.checkin_date,
                "day_index": checkin.day_index,
                "status": checkin.status,
                "notes": checkin.notes,
                "attachment_count": checkin.attachment_count,
                "attachment_hashes": attachment_hashes,
                "created_at": checkin.created_at,
            })
        })
        .collect();

    let summary_json = json!({
        "challenge_id": challenge_id,
        "participant_wallet": participant_wallet,
        "challenge_type": metadata.challenge_type,
        "metadata": {
            "challenge_pubkey": metadata.challenge_pubkey,
            "title": metadata.title,
            "description": metadata.description,
            "tags": metadata.tags,
            "proof_rule_config": metadata.proof_rule_config,
        },
        "rules": {
            "required_checkins": rules.required_checkins,
            "target_days": rules.target_days,
            "grace_days": rules.grace_days,
            "rules_json": rules.rules_json,
        },
        "submission": {
            "id": submission.id,
            "status": submission.status,
            "final_result_claim": submission.final_result_claim,
            "finalized_at": submission.finalized_at,
            "updated_at": submission.updated_at,
        },
        "evaluation": evaluation,
        "proof_files": proof_files_json,
        "daily_checkins": daily_checkins_json,
    });

    let summary_text = serde_json::to_string_pretty(&summary_json)?;
    let final_sha256 = hex::encode(Sha256::digest(summary_text.as_bytes()));

    let summary = sqlx::query_as::<_, FinalProofSummary>(
        r#"
        INSERT INTO final_proof_summaries (
            challenge_id, participant_wallet, challenge_type, summary_json, summary_text,
            final_sha256, based_on_submission_updated_at, generated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        ON CONFLICT (challenge_id, participant_wallet) DO UPDATE SET
            challenge_type = EXCLUDED.challenge_type,
            summary_json = EXCLUDED.summary_json,
            summary_text = EXCLUDED.summary_text,
            final_sha256 = EXCLUDED.final_sha256,
            based_on_submission_updated_at = EXCLUDED.based_on_submission_updated_at,
            generated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(challenge_id)
    .bind(participant_wallet)
    .bind(&metadata.challenge_type)
    .bind(&summary_json)
    .bind(&summary_text)
    .bind(&final_sha256)
    .bind(submission.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(summary)
}

fn evaluate_submission(
    challenge_type: &str,
    rules: &ChallengeRuleConfig,
    proof_files: &[ProofFile],
    daily_checkins: &[DailyCheckin],
) -> Value {
    let final_proof_count = proof_files
        .iter()
        .filter(|file| file.proof_kind == "final")
        .count() as i32;
    let completed_checkins = daily_checkins.len() as i32;
    let expected_checkins = rules.required_checkins.or(rules.target_days);
    let effective_completed = completed_checkins + rules.grace_days;

    let qualifies_success = match challenge_type {
        "final_only" => final_proof_count > 0,
        "daily_checkin" => match expected_checkins {
            Some(target) => effective_completed >= target,
            None => completed_checkins > 0 && final_proof_count > 0,
        },
        _ => false,
    };

    json!({
        "qualifies_success": qualifies_success,
        "final_proof_count": final_proof_count,
        "completed_checkins": completed_checkins,
        "expected_checkins": expected_checkins,
        "effective_completed_checkins": effective_completed,
        "grace_days_applied": rules.grace_days,
    })
}

pub fn validate_challenge_type(challenge_type: &str) -> anyhow::Result<()> {
    match challenge_type {
        "final_only" | "daily_checkin" => Ok(()),
        _ => bail!("Unsupported challenge_type"),
    }
}

fn sanitize_file_name(file_name: &str) -> String {
    file_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect()
}
