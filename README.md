# Pact Backend

Backend API for Pact, a Solana-powered accountability challenge platform. This service handles wallet authentication, challenge metadata, proof submission, submission summaries, dispute review payloads, and on-chain event indexing.

## Why This Matters

Pact challenges need more than a basic CRUD backend. The service has to connect off-chain product flows with on-chain challenge state in a way that is usable for mobile clients and reviewable by judges.

This backend is built to provide:

- Wallet-based authentication with signed nonces and JWT sessions
- Challenge metadata and rules storage for app-facing UX
- File-based proof ingestion with deterministic SHA-256 hashing
- Support for both `final_only` and `daily_checkin` challenge formats
- Submission summaries and dispute review payloads for verification workflows
- Solana event polling for syncing platform state with the chain

## Stack

- Rust
- Axum
- PostgreSQL
- SQLx
- JWT
- Solana RPC polling

## Core Capabilities

### 1. Wallet Authentication

Participants authenticate by signing a nonce message with their wallet. The backend verifies the Ed25519 signature and issues a JWT for protected routes.

### 2. Challenge Configuration Layer

The API stores challenge metadata that complements on-chain data:

- title and description
- tags
- challenge type
- proof rule configuration
- optional daily check-in rules

This keeps the client experience flexible without forcing every UX field on-chain.

### 3. Proof and Submission System

The backend supports two proof models:

- `final_only`: a participant submits final proof at the end of the challenge
- `daily_checkin`: a participant submits repeated check-ins plus a final summary

Uploaded files are stored locally, tracked in PostgreSQL, and hashed with SHA-256 so the client can reference deterministic proof artifacts.

### 4. Review and Dispute Support

For each participant submission, the API can generate:

- a normalized final summary
- a final proof hash
- a dispute review payload combining submission data, files, and check-ins

This is useful for moderation, dispute handling, or any judge-facing verification flow.

### 5. Chain-Aware Backend

The service exposes chain configuration needed by clients and runs a Solana event listener in the background to index relevant program activity.

## High-Level Architecture

```text
Mobile / Frontend
        |
        v
   Axum REST API
        |
        +--> PostgreSQL
        |
        +--> Local proof storage
        |
        +--> Solana RPC (devnet)
```

## API Overview

### Public Endpoints

| Method | Path | Purpose |
|------|------|------|
| GET | `/health` | Service and database health |
| GET | `/api/auth/nonce` | Generate wallet-signing nonce |
| POST | `/api/auth/verify` | Verify signature and issue JWT |
| GET | `/api/chain/config` | Return platform chain config |
| GET | `/api/challenges` | List challenges |
| GET | `/api/challenges/{challenge_id}` | Get one challenge |
| GET | `/api/challenges/{challenge_id}/config` | Get challenge metadata + rules |
| GET | `/api/challenges/{challenge_id}/submissions` | List submissions for a challenge |
| GET | `/api/challenges/{challenge_id}/submissions/{wallet}` | Get one participant submission |
| GET | `/api/challenges/{challenge_id}/submissions/{wallet}/summary` | Get generated final summary |
| POST | `/api/challenges/{challenge_id}/submissions/{wallet}/summary` | Regenerate final summary |
| GET | `/api/challenges/{challenge_id}/submissions/{wallet}/proof-hash` | Get final SHA-256 proof hash |
| GET | `/api/challenges/{challenge_id}/submissions/{wallet}/dispute-review` | Get review-ready dispute payload |
| GET | `/api/proofs/{challenge_id}/{wallet}` | Get legacy proof record |
| GET | `/api/events/{challenge_id}` | Get indexed on-chain events |

### Protected Endpoints

All protected routes require:

```http
Authorization: Bearer <jwt>
```

| Method | Path | Purpose |
|------|------|------|
| POST | `/api/chain/finalize-plan` | Build client-facing finalize plan |
| POST | `/api/challenges/{challenge_id}/metadata` | Create or update challenge metadata |
| POST | `/api/challenges/{challenge_id}/proofs/final` | Upload final proof |
| POST | `/api/challenges/{challenge_id}/checkins` | Upload daily check-in |
| POST | `/api/proofs/upload` | Legacy proof upload endpoint |

## Authentication Flow

```text
1. GET  /api/auth/nonce
2. Wallet signs the returned message
3. POST /api/auth/verify
4. Backend returns JWT
5. Client uses JWT for protected routes
```

## Environment Variables

Example `.env`:

```env
DATABASE_URL=postgres://pact:pact123@localhost:5432/pact_db
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=CvTggHr71Qm6NC5qjkvCq4txe2UbbZWrKAWGpMVMWw6y
PLATFORM_FEE_BPS=500
PLATFORM_CONFIG_PDA=REPLACE_WITH_PLATFORM_CONFIG_PDA
TREASURY_AUTHORITY=5UBYd69pmTayz8sKDWkhduLojrJe513qr1zHBoXet228
TREASURY_TOKEN_ACCOUNTS_JSON={}
JWT_SECRET=replace_with_a_secure_secret
PROOF_STORAGE_DIR=./proofs
PORT=8080
```

Key notes:

- `PROGRAM_ID`, `PLATFORM_CONFIG_PDA`, `TREASURY_AUTHORITY`, and `JWT_SECRET` are required
- `TREASURY_TOKEN_ACCOUNTS_JSON` must be valid JSON
- `PROOF_STORAGE_DIR` defaults to `./proofs`
- the current project configuration targets Solana devnet

## Local Setup

### Prerequisites

- Rust 1.75+
- PostgreSQL
- Git

### 1. Install dependencies

```bash
cargo build
```

### 2. Create the database

```bash
createdb pact_db
```

Or create it using your preferred PostgreSQL workflow, then make sure `DATABASE_URL` points to it.

### 3. Start the API

```bash
cargo run
```

Expected startup behavior:

- connects to PostgreSQL
- applies SQL migration statements from `migrations/001_init.sql`
- creates the proof storage directory if needed
- starts the Solana listener in the background
- serves HTTP on `0.0.0.0:$PORT`

### 4. Verify health

```bash
curl http://localhost:8080/health
```

Expected response shape:

```json
{
  "status": "healthy",
  "database": true,
  "program_id": "CvTggHr71Qm6NC5qjkvCq4txe2UbbZWrKAWGpMVMWw6y"
}
```

## Project Structure

```text
pact-backend/
├── Cargo.toml
├── migrations/
│   └── 001_init.sql
├── proofs/
└── src/
    ├── main.rs
    ├── config.rs
    ├── db/
    ├── middleware/
    ├── models/
    ├── routes/
    └── services/
```

## Notes for Judges

This backend is designed around a practical Web2 + Web3 split:

- immutable challenge settlement and economic logic can live on-chain
- product-facing metadata, uploads, summaries, and review workflows live off-chain
- the API bridges both sides in a form that mobile clients can use directly

The result is a backend that is not just a storage layer, but an integration layer for challenge operations, evidence handling, and chain-aware user flows.

## Current Limitations

- proof files are stored on local disk, not object storage
- Solana indexing currently depends on RPC availability
- the legacy `/api/proofs/upload` route coexists with the newer submission flow

## Next Improvements

- move proof storage to S3-compatible object storage
- add richer observability and structured metrics
- strengthen dispute automation and scoring reconciliation
- expand test coverage around submission and chain-sync flows
