# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Whisper-Wish** is a local subtitle generation and translation system. It combines whisper-cpp for speech-to-text with NLLB-200 neural translation, integrated with Sonarr/Radarr for automatic media library processing. Everything runs on macOS Apple Silicon.

Two main components:
- **SubsForge** (`subsforge/`) — Rust app: Axum API server + Sonarr/Radarr watcher + pipeline orchestrator
- **Translator** (`subsforge/translator/`) — Python FastAPI server wrapping NLLB-200 for translation

Plus a standalone whisper-cpp CLI wrapper (`whisper.sh`) and Raycast scripts.

## Build & Run

```bash
# Rust (SubsForge)
cd subsforge
cargo build                    # debug build
cargo build --release          # release build
cargo test                     # run tests (SRT parsing, naming)

# Python (Translator)
cd subsforge/translator
uv sync                        # install dependencies
uv run uvicorn server:app --host 0.0.0.0 --port 8384  # start server

# Run SubsForge
cd subsforge
cargo run -- serve -c config.toml       # API server + watcher daemon
cargo run -- process -c config.toml FILE # process single file
```

## Architecture

```
Sonarr/Radarr (remote server)
    │  polling via REST API
    ▼
┌─────────────────────────────────────────────────┐
│  SubsForge (Rust, port 8385)                    │
│                                                 │
│  watcher/ ──→ pipeline/ ──→ api/                │
│  poll Sonarr    ffmpeg.rs     Axum REST         │
│  poll Radarr    whisper.rs    GET/POST /api/*   │
│                 translator.rs                   │
│                 srt.rs                          │
│                    │                            │
│  db.rs ◄───────────┘  SQLite (sqlx)            │
└────────────────────┬────────────────────────────┘
                     │ HTTP POST /translate
                     ▼
┌─────────────────────────────────────────────────┐
│  Translator (Python FastAPI, port 8384)         │
│  facebook/nllb-200-distilled-600M               │
│  MPS (Metal GPU) or CPU                         │
└─────────────────────────────────────────────────┘
```

**Pipeline per media file:**
`pending` → `extracting` (ffmpeg → WAV 16kHz mono) → `transcribing` (whisper-cli → SRT) → `translating` (NLLB-200 per target lang) → `completed`

Output: `filename.fr.srt` + `filename.en.srt` alongside the media file (Plex/Jellyfin naming).

## Key Design Decisions

- **SRT translation strategy**: Only subtitle text is sent to the translator (timestamps/sequence numbers stay in Rust). Zero risk of breaking SRT formatting.
- **NLLB-200 language codes**: Mapped from ISO 639-1 in `src/pipeline/translator.rs` (e.g., `"en"` → `"eng_Latn"`, `"ja"` → `"jpn_Jpan"`).
- **Path mapping**: Remote Sonarr/Radarr paths are converted to local mount points via `[[path_mappings]]` in config.
- **Database**: SQLite with raw SQL migrations (no sqlx macros, runtime queries only). Schema in `migrations/001_init.sql`.
- **Watcher creates jobs with INSERT OR IGNORE**: Idempotent — re-polling the same media won't create duplicates.

## External Dependencies (not in repo)

- `whisper-cli` / `whisper-stream` — installed via `brew install whisper-cpp`, model at `~/02_perso/whisper/models/ggml-large-v3-turbo.bin`
- `ffmpeg` — installed via Homebrew
- `uv` — Python package manager for the translator
- Sonarr/Radarr — running on a remote server, accessed via REST API
- Media files — accessible via SMB/NFS mount

## Config

Copy `config.example.toml` to `config.toml` and fill in Sonarr/Radarr API keys and path mappings. Config with secrets is gitignored.

## API Endpoints (Axum, port 8385)

```
GET  /api/health          → { whisper, translator, database } status
GET  /api/jobs?status=&limit=  → list jobs
POST /api/jobs            → create manual job { media_path, title }
GET  /api/jobs/{id}       → job detail with translations
POST /api/jobs/{id}/retry → retry failed job
GET  /api/stats           → counts by status
```

## Translator API (FastAPI, port 8384)

```
POST /translate  → { text: [...], source_lang: "eng_Latn", target_lang: "fra_Latn" }
GET  /health     → { status, model, device }
```
