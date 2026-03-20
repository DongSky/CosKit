# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CosKit is an AI-driven portrait retouching desktop app built with **Rust + Tauri v2** and a vanilla HTML/CSS/JS frontend (no framework). It uses the Google Gemini API for intelligent photo editing, background replacement, and Cosplay effects.

## Build & Development Commands

```bash
npm install                # Install Tauri CLI (first time)
npx tauri dev              # Dev mode with hot reload
npx tauri build            # Production build (DMG on macOS, MSI/EXE on Windows)
cargo check -p coskit      # Type-check Rust backend only
cargo build -p coskit      # Build Rust backend only
```

Platform-specific build scripts: `build_mac.sh`, `build_win.bat`.

There are no automated tests. Testing is manual via the dev mode UI.

## Architecture

### Backend (`src-tauri/src/`)

- **`engine.rs`** — Core orchestration: session lifecycle, tree-based edit history (DAG of `EditNode`s), and the modular AI pipeline (`run_modular_pipeline`: scene detection → background analysis → retouch → effects).
- **`commands.rs`** — Tauri IPC command handlers. Every `#[tauri::command]` fn bridges frontend calls to engine/settings logic.
- **`gemini_client.rs`** — Gemini API integration. Singleton `GeminiClients` via `OnceLock`. Handles text + image model endpoints separately, with retry/exponential backoff. Key functions: `detect_scene_type`, `analyze_background`, `retouch_image`, `apply_cosplay_effect`.
- **`models.rs`** — Core data structures: `EditNode`, `Session`, `Settings`, `PipelineModules`, `ReferenceImage`.
- **`settings.rs`** — Platform-aware config persistence (`~/.coskit/` or equivalent). Default prompt templates with `{{KEYWORD_HINT}}` / `{{REFERENCE_IMAGES_HINT}}` template variables.
- **`image_utils.rs`** — JPEG encode/decode, thumbnail generation (512px), base64 conversion, resize with Lanczos3.
- **`dotenv.rs`** — Multi-location `.env` loading (data dir → exe dir → exe parent → home dir).

### Frontend (`src/`)

- **`app.js`** — Single-page app: chat-style UI, session management, polling for background task completion (500ms interval), settings modal, reference image management.
- **`index.html`** — Semantic HTML structure with modals for settings/history/help.
- **`style.css`** — Dark theme with CSS custom properties.

Frontend communicates with backend exclusively through `window.__TAURI__.core.invoke()` calls.

### Key Patterns

- **Edit tree (DAG)**: Sessions contain a tree of `EditNode`s. Users branch from any historical node. `active_path` tracks the current branch.
- **Async edit processing**: `submit_edit()` spawns a `tokio::spawn()` background task. Frontend polls `get_node_status()` until completion. Node status: `pending → processing → done/error`.
- **Thread-safe state**: Sessions stored in `Arc<RwLock<HashMap>>` managed by Tauri state.
- **Singleton AI clients**: `OnceLock<GeminiClients>` with lazy init, merging app settings > env vars > defaults.
- **Image data flow**: Images transmitted as base64-encoded JPEG via JSON. Reference images resized to 1024px before API submission.

## Configuration

API keys can be set via Settings UI (persisted to `settings.json`) or `.env` files with `GEMINI_API_KEY`, `GEMINI_BASE_URL`, `GEMINI_IMAGE_BASE_URL`, `GEMINI_TEXT_MODEL`, `GEMINI_IMAGE_MODEL`.
