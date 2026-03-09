use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ParticipantSubmission {
    pub id: Uuid,
    pub challenge_id: String,
    pub participant_wallet: String,
    pub challenge_type: String,
    pub status: String,
    pub final_result_claim: Option<String>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct DailyCheckin {
    pub id: Uuid,
    pub challenge_id: String,
    pub participant_wallet: String,
    pub submission_id: Uuid,
    pub checkin_date: NaiveDate,
    pub day_index: Option<i32>,
    pub status: String,
    pub notes: String,
    pub attachment_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProofFile {
    pub id: Uuid,
    pub challenge_id: String,
    pub participant_wallet: String,
    pub submission_id: Option<Uuid>,
    pub checkin_id: Option<Uuid>,
    pub challenge_type: String,
    pub proof_kind: String,
    pub storage_path: String,
    pub file_url: String,
    pub file_name: String,
    pub mime_type: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FinalProofSummary {
    pub id: Uuid,
    pub challenge_id: String,
    pub participant_wallet: String,
    pub challenge_type: String,
    pub summary_json: Value,
    pub summary_text: String,
    pub final_sha256: String,
    pub based_on_submission_updated_at: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SubmissionListQuery {
    pub participant_wallet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParticipantSubmissionDetailResponse {
    pub submission: ParticipantSubmission,
    pub proof_files: Vec<ProofFile>,
    pub daily_checkins: Vec<DailyCheckin>,
}

#[derive(Debug, Serialize)]
pub struct FinalProofHashResponse {
    pub challenge_id: String,
    pub participant_wallet: String,
    pub challenge_type: String,
    pub final_sha256: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DisputeReviewResponse {
    pub challenge_id: String,
    pub participant_wallet: String,
    pub challenge_type: String,
    pub submission: ParticipantSubmission,
    pub summary: FinalProofSummary,
    pub proof_files: Vec<ProofFile>,
    pub daily_checkins: Vec<DailyCheckin>,
}
