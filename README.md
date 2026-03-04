# Pact Backend — Setup & Run Guide

REST API backend untuk Pact. Menangani autentikasi wallet, metadata challenge, upload bukti, sistem skor, dan indexing event on-chain dari Solana.

**Tech Stack:** Rust (Axum 0.7) + PostgreSQL 16 + SQLx + JWT + Solana RPC polling

---

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| **Rust** | 1.75+ | https://rustup.rs |
| **PostgreSQL** | 16 | Lihat langkah di bawah |
| **Git** | Any | https://git-scm.com |
| **ngrok** | 3.20+ | https://ngrok.com/download |

> Semua langkah di bawah diasumsikan berjalan di **WSL 2 (Ubuntu 24.04)**. Jangan jalankan backend di Windows native.

---

## 1. Clone & Masuk ke Direktori

```bash
git clone <repo-url>
cd pact-app/pact-backend
```

---

## 2. Install PostgreSQL

PostgreSQL sudah terinstall di mesin original. Jika setup di mesin baru:

```bash
sudo apt update
sudo apt install -y postgresql postgresql-contrib
```

### Start PostgreSQL service

```bash
sudo service postgresql start
```

Verifikasi berjalan:

```bash
pg_isready -h localhost -p 5433
# Expected: localhost:5433 - accepting connections
```

> **Catatan port:** PostgreSQL cluster berjalan di port **5433**, bukan 5432 default. Ini karena konfigurasi cluster Ubuntu.

### Buat database dan user

```bash
sudo -u postgres psql -p 5433 <<EOF
CREATE USER pact WITH PASSWORD 'pact123';
CREATE DATABASE pact_db OWNER pact;
GRANT ALL PRIVILEGES ON DATABASE pact_db TO pact;
EOF
```

Verifikasi koneksi:

```bash
psql "postgres://pact:pact123@localhost:5433/pact_db" -c "\dt"
```

Jika database baru, tabel belum ada — itu normal. Migrasi dijalankan otomatis saat `cargo run`.

---

## 3. Environment Setup

File `.env` sudah ada di repo. Pastikan isinya seperti ini (jangan ubah kecuali perlu):

```env
DATABASE_URL=postgres://pact:pact123@localhost:5433/pact_db
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=6JTfaG74DZydUwHdQAo6P6frYAVhSitwTexomvTphbCs
JWT_SECRET=faf07ec6bcb8e1e438b407c882fbc5d8ddce44060e8aaf581e81a09668f32c60
PORT=8080
```

> **JWT_SECRET** — jaga kerahasiaan ini. Jangan ganti nilainya atau semua token yang sudah di-issue akan invalid.

---

## 4. Build & Run

```bash
# Dari direktori pact-backend/
cargo run
```

Build pertama kali membutuhkan waktu ~2–3 menit (download + compile dependencies). Selanjutnya jauh lebih cepat.

Output yang diharapkan:
```
INFO pact_backend: Connected to database
INFO pact_backend: Migrations applied
INFO pact_backend: Solana event listener spawned
INFO pact_backend: Pact backend starting on 0.0.0.0:8080
```

Verifikasi backend hidup:

```bash
curl http://localhost:8080/health
# {"status":"healthy","database":true,"program_id":"6JTfaG...","solana_rpc":"..."}
```

---

## 5. Expose Backend via ngrok (untuk frontend di Windows)

Frontend React Native di Windows tidak bisa akses `localhost` WSL langsung. Gunakan ngrok sebagai tunnel.

### Install ngrok (WSL)

```bash
curl -sSL https://ngrok-agent.s3.amazonaws.com/ngrok.asc | sudo tee /etc/apt/trusted.gpg.d/ngrok.asc >/dev/null
echo "deb https://ngrok-agent.s3.amazonaws.com buster main" | sudo tee /etc/apt/sources.list.d/ngrok.list
sudo apt update && sudo apt install ngrok
```

Atau download binary langsung:

```bash
wget https://bin.equinox.io/c/bNyj1mQVY4c/ngrok-v3-stable-linux-amd64.tgz
tar -xzf ngrok-v3-stable-linux-amd64.tgz
mv ngrok ~/.local/bin/
```

### Login ngrok (perlu sekali)

```bash
ngrok config add-authtoken <YOUR_AUTHTOKEN>
```

Dapatkan authtoken di: https://dashboard.ngrok.com/get-started/your-authtoken

### Jalankan tunnel

```bash
# Pastikan backend sudah running di port 8080 dulu
ngrok http 8080
```

Ambil URL dari output, contoh:
```
Forwarding  https://xxxx-xxxx.ngrok-free.app -> http://localhost:8080
```

URL itulah yang dipakai frontend. Update `src/utils/constants.js` di pact-mobile:

```js
export const API_BASE_URL = "https://xxxx-xxxx.ngrok-free.app";
```

> **Penting:** URL ngrok berubah setiap kali ngrok di-restart (free tier). Kalau frontend tiba-tiba tidak bisa connect, kemungkinan URL ngrok sudah berganti.

---

## 6. API Endpoints

### Public (tidak perlu auth)

| Method | Path | Deskripsi |
|--------|------|-----------|
| GET | `/health` | Status backend, DB, dan program |
| GET | `/api/auth/nonce` | Minta nonce untuk signing |
| POST | `/api/auth/verify` | Verifikasi signature wallet → JWT |
| GET | `/api/challenges` | List semua challenges (paginated) |
| GET | `/api/challenges/:id` | Detail satu challenge |
| GET | `/api/proofs/:challenge_id/:wallet` | Ambil data proof |
| GET | `/api/scores/:wallet` | Skor satu wallet |
| GET | `/api/scores` | Leaderboard (paginated) |
| GET | `/api/events/:challenge_id` | Event on-chain per challenge |

### Protected (perlu header `Authorization: Bearer <token>`)

| Method | Path | Deskripsi |
|--------|------|-----------|
| POST | `/api/challenges/:id/metadata` | Simpan/update metadata challenge |
| POST | `/api/proofs/upload` | Upload file bukti (multipart form) |

### Auth Flow

```
1. GET /api/auth/nonce           → { nonce, message }
2. Sign "message" dengan wallet (Mobile Wallet Adapter)
3. POST /api/auth/verify         → { token, wallet_address }
4. Simpan token, kirim sebagai: Authorization: Bearer <token>
```

Token valid selama **24 jam**.

---

## 7. Cara Kerja Sistem Skor

Backend memonitor on-chain events dari program Solana setiap **10 detik**. Saat challenge selesai (`finalize` instruction), backend otomatis:

- Fetch semua akun `ParticipantState` dari transaksi
- Deserialize data on-chain
- Update skor di database

| Kondisi | Perubahan Skor |
|---------|---------------|
| Menang challenge | +10, streak +1 |
| Kalah (submit Fail) | -15, streak reset |
| Kalah (di-dispute) | -20, streak reset |
| Kalah (tidak submit) | -25, streak reset |

Skor minimum adalah **0** (tidak bisa negatif).

---

## 8. Project Structure

```
pact-backend/
├── .env                        # Konfigurasi (database, RPC, JWT, port)
├── Cargo.toml                  # Dependencies Rust
├── migrations/
│   └── 001_init.sql            # Schema database (dijalankan otomatis)
└── src/
    ├── main.rs                 # Entry point, router, spawns event listener
    ├── config.rs               # AppState struct (DB pool, RPC URL, JWT secret)
    ├── db/
    │   └── mod.rs              # Fungsi DB utility (health ping)
    ├── middleware/
    │   └── auth.rs             # JWT validation middleware (require_auth)
    ├── models/
    │   ├── challenge.rs        # ChallengeMetadata struct
    │   ├── event.rs            # EventLog struct
    │   ├── participant.rs      # ParticipantInfo struct
    │   ├── proof.rs            # Proof struct
    │   └── score.rs            # CommitmentScore struct
    ├── routes/
    │   ├── auth.rs             # GET /nonce, POST /verify
    │   ├── challenges.rs       # GET/POST challenge endpoints
    │   ├── events.rs           # GET events per challenge
    │   ├── health.rs           # GET /health
    │   ├── proofs.rs           # POST upload, GET proof
    │   └── scores.rs           # GET score, GET leaderboard
    └── services/
        ├── scoring.rs          # Logic update skor (complete/fail)
        └── solana_listener.rs  # Poll Solana RPC, index events, trigger scoring
```

---

## 9. Menambahkan Endpoint Baru

Pola yang digunakan:

1. **Tambah model** di `src/models/` (struct + `#[derive(sqlx::FromRow, serde::Serialize)]`)
2. **Tambah route handler** di `src/routes/` (async fn dengan `State<AppState>`)
3. **Daftarkan route** di `src/main.rs` (`.route("/api/...", get/post(...))`)
4. Jika perlu tabel baru, tambahkan SQL ke `migrations/001_init.sql` — migrasi dijalankan otomatis saat startup

---

## 10. Troubleshooting

| Problem | Solusi |
|---------|--------|
| `Address already in use (os error 98)` | Port 8080 sudah dipakai proses lain. Jalankan: `lsof -ti :8080 \| xargs kill -9` |
| `Connection refused` ke DB | PostgreSQL belum jalan: `sudo service postgresql start` |
| `authentication failed` di DB | User `pact` belum dibuat. Lihat langkah 2. |
| Cargo build error: `linker not found` | Install build tools: `sudo apt install build-essential` |
| `ngrok: version too old` | Jangan gunakan ngrok dari winget/chocolatey. Install di WSL langsung (lihat langkah 5). |
| Frontend dapat 502 dari ngrok | Backend tidak jalan. Pastikan `cargo run` sudah aktif. |
| Scoring tidak update setelah settle | Event listener polling 10 detik. Tunggu ~30 detik setelah finalize tx. |

---

## 11. Key Config Values

| Nilai | Keterangan |
|-------|-----------|
| `PROGRAM_ID` | `6JTfaG74DZydUwHdQAo6P6frYAVhSitwTexomvTphbCs` — Anchor program di devnet |
| `SOLANA_RPC_URL` | `https://api.devnet.solana.com` — jangan ganti ke mainnet |
| `DATABASE_URL` | `postgres://pact:pact123@localhost:5433/pact_db` |
| `PORT` | `8080` — port backend |
| `JWT_SECRET` | Token signing secret — jangan ganti |
