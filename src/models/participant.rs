use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub wallet_address: String,
    pub deposited: bool,
    pub submitted: bool,
    pub submission_result: String,
    pub proof_hash: String,
    pub dispute_count: u8,
    pub is_winner: bool,
}