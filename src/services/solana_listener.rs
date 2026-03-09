use serde_json::json;
use sqlx::PgPool;

/// Solana event listener — polls for program transactions and indexes them.
/// Uses getSignaturesForAddress + getTransaction to parse Anchor event logs.
pub async fn poll_program_events(pool: &PgPool, rpc_url: &str, program_id: &str) {
    tracing::info!("Starting Solana event listener for program: {}", program_id);

    let client = reqwest::Client::new();
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        interval.tick().await;

        match fetch_recent_signatures(&client, rpc_url, program_id).await {
            Ok(sig_entries) => {
                for entry in sig_entries {
                    let exists: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM event_log WHERE signature = $1)",
                    )
                    .bind(&entry.signature)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(true);

                    if exists {
                        continue;
                    }

                    if let Err(e) =
                        index_transaction(pool, &client, rpc_url, &entry).await
                    {
                        tracing::warn!("Failed to index tx {}: {}", entry.signature, e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch signatures: {}", e);
            }
        }
    }
}

struct SigEntry {
    signature: String,
    slot: u64,
    block_time: Option<i64>,
}

async fn fetch_recent_signatures(
    client: &reqwest::Client,
    rpc_url: &str,
    program_id: &str,
) -> anyhow::Result<Vec<SigEntry>> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSignaturesForAddress",
        "params": [program_id, {"limit": 20}]
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let entries = resp["result"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let sig = item["signature"].as_str()?.to_string();
                    let slot = item["slot"].as_u64().unwrap_or(0);
                    let block_time = item["blockTime"].as_i64();
                    Some(SigEntry {
                        signature: sig,
                        slot,
                        block_time,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(entries)
}

/// Anchor emits events as base64-encoded data in program log messages
/// prefixed with "Program data: ". The first 8 bytes are the event discriminator.
/// We detect known event names from log lines like "Program log: ..." to classify events.
async fn index_transaction(
    pool: &PgPool,
    client: &reqwest::Client,
    rpc_url: &str,
    entry: &SigEntry,
) -> anyhow::Result<()> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            entry.signature,
            {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}
        ]
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let tx = &resp["result"];
    if tx.is_null() {
        return Ok(());
    }

    let log_messages = tx["meta"]["logMessages"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let event_type = detect_event_type(&log_messages);
    let err = tx["meta"]["err"].clone();
    let is_success = err.is_null();

    let account_keys = tx["transaction"]["message"]["accountKeys"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let wallet_address = account_keys
        .first()
        .and_then(|k| {
            k["pubkey"]
                .as_str()
                .or_else(|| k.as_str())
                .map(String::from)
        });

    let block_time = entry
        .block_time
        .map(|ts| chrono::DateTime::from_timestamp(ts, 0));

    let data = json!({
        "success": is_success,
        "err": err,
        "log_count": log_messages.len(),
    });

    sqlx::query(
        r#"
        INSERT INTO event_log (signature, event_type, wallet_address, data, slot, block_time)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (signature) DO NOTHING
        "#,
    )
    .bind(&entry.signature)
    .bind(&event_type)
    .bind(&wallet_address)
    .bind(&data)
    .bind(entry.slot as i64)
    .bind(block_time.flatten())
    .execute(pool)
    .await?;

    tracing::info!("Indexed tx {} as {}", entry.signature, event_type);

    // Local score projection is disabled for now.
    // Commitment score should come from on-chain CommitmentProfile instead.
    if is_success && event_type == "ChallengeSettled" {
        let participant_count = account_keys
            .iter()
            .filter_map(|k| {
                k["pubkey"]
                    .as_str()
                    .or_else(|| k.as_str())
                    .map(String::from)
            })
            .count();

        tracing::info!(
            "ChallengeSettled indexed for {} accounts; skipping backend-local score update",
            participant_count
        );
    }

    Ok(())
}

/// Detect Anchor event type from program log messages.
fn detect_event_type(logs: &[serde_json::Value]) -> String {
    for log in logs {
        let msg = match log.as_str() {
            Some(s) => s,
            None => continue,
        };

        if let Some(rest) = msg.strip_prefix("Program log: Instruction: ") {
            return match rest.trim() {
                "CreateChallenge" => "ChallengeCreated",
                "JoinChallenge" => "ParticipantJoined",
                "StartChallenge" => "ChallengeStarted",
                "SubmitResult" => "ResultSubmitted",
                "Dispute" => "DisputeFiled",
                "Finalize" => "ChallengeSettled",
                "VoteContinue" => "VoteCast",
                other => other,
            }
            .to_string();
        }
    }

    "unknown".to_string()
}

// --- On-chain account deserialization for scoring ---

// Anchor discriminators (first 8 bytes of sha256("account:<Name>"))
const CHALLENGE_DISCRIMINATOR: [u8; 8] = [119, 250, 161, 121, 119, 81, 22, 208];
const PARTICIPANT_DISCRIMINATOR: [u8; 8] = [94, 173, 138, 206, 211, 209, 252, 75];

/// Minimal Challenge fields we need for scoring
struct OnChainChallenge {
    stake_amount: u64,
}

/// Minimal ParticipantState fields we need for scoring
struct OnChainParticipant {
    participant: [u8; 32], // pubkey
    deposited: bool,
    submitted: bool,
    submission_result: u8, // 0=None, 1=Success, 2=Fail
    disputed: bool,
    dispute_count: u8,
    is_winner: bool,
}

fn parse_challenge(data: &[u8]) -> Option<OnChainChallenge> {
    if data.len() < 8 || data[..8] != CHALLENGE_DISCRIMINATOR {
        return None;
    }
    let d = &data[8..];
    // Layout: creator(32) + challenge_id(4+len) + title(4+len) + stake_amount(8) + ...
    // Skip creator (32 bytes)
    let pos = 32;
    if d.len() < pos + 4 {
        return None;
    }
    // Skip challenge_id string (4 byte len prefix + content)
    let id_len = u32::from_le_bytes(d[pos..pos + 4].try_into().ok()?) as usize;
    let pos = pos + 4 + id_len;
    if d.len() < pos + 4 {
        return None;
    }
    // Skip title string (4 byte len prefix + content)
    let title_len = u32::from_le_bytes(d[pos..pos + 4].try_into().ok()?) as usize;
    let pos = pos + 4 + title_len;
    if d.len() < pos + 8 {
        return None;
    }
    let stake_amount = u64::from_le_bytes(d[pos..pos + 8].try_into().ok()?);

    Some(OnChainChallenge { stake_amount })
}

fn parse_participant(data: &[u8]) -> Option<OnChainParticipant> {
    if data.len() < 8 || data[..8] != PARTICIPANT_DISCRIMINATOR {
        return None;
    }
    let d = &data[8..];
    // Layout: participant(32) + challenge(32) + deposited(1) + submitted(1)
    //         + submission_result(1) + proof_hash(32) + disputed(1) + dispute_count(1)
    //         + is_winner(1) + ...
    if d.len() < 32 + 32 + 1 + 1 + 1 + 32 + 1 + 1 + 1 {
        return None;
    }
    let participant: [u8; 32] = d[0..32].try_into().ok()?;
    // skip challenge pubkey (32)
    let deposited = d[64] != 0;
    let submitted = d[65] != 0;
    let submission_result = d[66]; // enum: 0=None, 1=Success, 2=Fail
    // skip proof_hash (32 bytes at offset 67)
    let disputed = d[99] != 0;
    let dispute_count = d[100];
    let is_winner = d[101] != 0;

    Some(OnChainParticipant {
        participant,
        deposited,
        submitted,
        submission_result,
        disputed,
        dispute_count,
        is_winner,
    })
}

/// Deprecated local score projection. Left in place until CommitmentProfile-based
/// score reads fully replace the legacy backend-local table.
async fn process_settlement_scoring(
    _pool: &PgPool,
    _client: &reqwest::Client,
    _rpc_url: &str,
    _account_keys: &[String],
) -> anyhow::Result<()> {
    tracing::info!("Skipping deprecated backend-local settlement scoring");
    Ok(())
}
