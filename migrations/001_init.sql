CREATE TABLE IF NOT EXISTS challenge_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL UNIQUE,
    challenge_pubkey VARCHAR(64) NOT NULL,
    title VARCHAR(128) NOT NULL,
    description TEXT DEFAULT '',
    tags TEXT[] DEFAULT '{}',
    challenge_type VARCHAR(32) NOT NULL DEFAULT 'final_only',
    proof_rule_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE challenge_metadata
    ADD COLUMN IF NOT EXISTS challenge_type VARCHAR(32) NOT NULL DEFAULT 'final_only';

ALTER TABLE challenge_metadata
    ADD COLUMN IF NOT EXISTS proof_rule_config JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE TABLE IF NOT EXISTS challenge_rules (
    challenge_id VARCHAR(64) PRIMARY KEY REFERENCES challenge_metadata(challenge_id) ON DELETE CASCADE,
    required_checkins INTEGER,
    target_days INTEGER,
    grace_days INTEGER NOT NULL DEFAULT 0,
    rules_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS commitment_scores (
    wallet_address VARCHAR(64) PRIMARY KEY,
    score INTEGER NOT NULL DEFAULT 100,
    challenges_joined INTEGER NOT NULL DEFAULT 0,
    challenges_completed INTEGER NOT NULL DEFAULT 0,
    challenges_failed INTEGER NOT NULL DEFAULT 0,
    challenges_disputed INTEGER NOT NULL DEFAULT 0,
    total_staked_lamports BIGINT NOT NULL DEFAULT 0,
    total_earned_lamports BIGINT NOT NULL DEFAULT 0,
    streak_current INTEGER NOT NULL DEFAULT 0,
    streak_best INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS proofs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL,
    wallet_address VARCHAR(64) NOT NULL,
    proof_hash VARCHAR(128) NOT NULL,
    file_path TEXT NOT NULL,
    file_name VARCHAR(256) NOT NULL DEFAULT '',
    content_type VARCHAR(64) NOT NULL DEFAULT 'application/octet-stream',
    file_size_bytes BIGINT NOT NULL DEFAULT 0,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(challenge_id, wallet_address)
);

CREATE TABLE IF NOT EXISTS participant_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL REFERENCES challenge_metadata(challenge_id) ON DELETE CASCADE,
    participant_wallet VARCHAR(64) NOT NULL,
    challenge_type VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    final_result_claim VARCHAR(16),
    finalized_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(challenge_id, participant_wallet)
);

CREATE TABLE IF NOT EXISTS daily_checkins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL REFERENCES challenge_metadata(challenge_id) ON DELETE CASCADE,
    participant_wallet VARCHAR(64) NOT NULL,
    submission_id UUID NOT NULL REFERENCES participant_submissions(id) ON DELETE CASCADE,
    checkin_date DATE NOT NULL,
    day_index INTEGER,
    status VARCHAR(32) NOT NULL DEFAULT 'submitted',
    notes TEXT DEFAULT '',
    attachment_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(challenge_id, participant_wallet, checkin_date)
);

CREATE TABLE IF NOT EXISTS proof_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL REFERENCES challenge_metadata(challenge_id) ON DELETE CASCADE,
    participant_wallet VARCHAR(64) NOT NULL,
    submission_id UUID REFERENCES participant_submissions(id) ON DELETE CASCADE,
    checkin_id UUID REFERENCES daily_checkins(id) ON DELETE CASCADE,
    challenge_type VARCHAR(32) NOT NULL,
    proof_kind VARCHAR(32) NOT NULL,
    storage_path TEXT NOT NULL,
    file_url TEXT NOT NULL,
    file_name VARCHAR(256) NOT NULL DEFAULT '',
    mime_type VARCHAR(128) NOT NULL DEFAULT 'application/octet-stream',
    sha256 VARCHAR(64) NOT NULL,
    size_bytes BIGINT NOT NULL DEFAULT 0,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS final_proof_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL REFERENCES challenge_metadata(challenge_id) ON DELETE CASCADE,
    participant_wallet VARCHAR(64) NOT NULL,
    challenge_type VARCHAR(32) NOT NULL,
    summary_json JSONB NOT NULL,
    summary_text TEXT NOT NULL,
    final_sha256 VARCHAR(64) NOT NULL,
    based_on_submission_updated_at TIMESTAMPTZ NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(challenge_id, participant_wallet)
);

CREATE TABLE IF NOT EXISTS event_log (
    id BIGSERIAL PRIMARY KEY,
    signature VARCHAR(128) NOT NULL UNIQUE,
    event_type VARCHAR(64) NOT NULL,
    challenge_id VARCHAR(64),
    wallet_address VARCHAR(64),
    data JSONB DEFAULT '{}',
    slot BIGINT NOT NULL DEFAULT 0,
    block_time TIMESTAMPTZ,
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_challenge_meta_pubkey ON challenge_metadata(challenge_pubkey);
CREATE INDEX IF NOT EXISTS idx_challenge_meta_creator ON challenge_metadata(created_by);
CREATE INDEX IF NOT EXISTS idx_challenge_meta_type ON challenge_metadata(challenge_type);
CREATE INDEX IF NOT EXISTS idx_challenge_rules_target_days ON challenge_rules(target_days);
CREATE INDEX IF NOT EXISTS idx_scores_ranking ON commitment_scores(score DESC);
CREATE INDEX IF NOT EXISTS idx_proofs_challenge ON proofs(challenge_id);
CREATE INDEX IF NOT EXISTS idx_proofs_wallet ON proofs(wallet_address);
CREATE INDEX IF NOT EXISTS idx_submissions_challenge ON participant_submissions(challenge_id);
CREATE INDEX IF NOT EXISTS idx_submissions_wallet ON participant_submissions(participant_wallet);
CREATE INDEX IF NOT EXISTS idx_checkins_challenge_wallet ON daily_checkins(challenge_id, participant_wallet);
CREATE INDEX IF NOT EXISTS idx_proof_files_submission ON proof_files(submission_id);
CREATE INDEX IF NOT EXISTS idx_proof_files_checkin ON proof_files(checkin_id);
CREATE INDEX IF NOT EXISTS idx_final_summaries_challenge_wallet ON final_proof_summaries(challenge_id, participant_wallet);
CREATE INDEX IF NOT EXISTS idx_events_challenge ON event_log(challenge_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON event_log(event_type);
CREATE INDEX IF NOT EXISTS idx_events_wallet ON event_log(wallet_address);
