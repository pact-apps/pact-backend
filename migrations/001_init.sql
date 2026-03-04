CREATE TABLE IF NOT EXISTS challenge_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id VARCHAR(64) NOT NULL UNIQUE,
    challenge_pubkey VARCHAR(64) NOT NULL,
    title VARCHAR(128) NOT NULL,
    description TEXT DEFAULT '',
    tags TEXT[] DEFAULT '{}',
    created_by VARCHAR(64) NOT NULL,
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
CREATE INDEX IF NOT EXISTS idx_scores_ranking ON commitment_scores(score DESC);
CREATE INDEX IF NOT EXISTS idx_proofs_challenge ON proofs(challenge_id);
CREATE INDEX IF NOT EXISTS idx_proofs_wallet ON proofs(wallet_address);
CREATE INDEX IF NOT EXISTS idx_events_challenge ON event_log(challenge_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON event_log(event_type);
CREATE INDEX IF NOT EXISTS idx_events_wallet ON event_log(wallet_address);