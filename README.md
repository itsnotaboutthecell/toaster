# Toaster

Toaster is a transcript-first desktop editor for spoken audio/video: **edit media by editing text**.

Forked from Handy, Toaster keeps the local-first model management and adds word-level edit workflows for media cleanup and export.

## What Toaster does today

- Open audio/video files and generate transcripts with local models
- Select words and apply non-destructive edit actions (delete/silence/restore/split)
- Keep transcript, waveform, and playback synchronized while editing
- Detect filler words and pauses for faster cleanup
- Export cleaned media plus captions (SRT/VTT) and script text
- Save/load project state for iterative editing sessions

## Tech stack

- **Desktop shell:** Tauri 2.x
- **Backend:** Rust (`src-tauri/`)
- **Frontend:** React + TypeScript + Tailwind (`src/`)
- **State:** Zustand stores
- **Transcription:** local model inference via `transcribe-rs`/whisper ecosystem

## Architecture (high level)

```text
Frontend (React + Zustand)
  -> Tauri commands
Backend (Rust managers)
  -> audio/model/transcription/editor/media/filler/history/export/project domains
```

Core rule: backend managers own business logic; frontend orchestrates UI and invokes commands.

## Quick start (development)

See [BUILD.md](BUILD.md) for platform setup.

### 1. Install deps

```bash
bun install --frozen-lockfile
```

### 2. Windows environment setup (required on Windows)

```powershell
.\scripts\setup-env.ps1
```

### 3. Run app

```bash
npm run tauri dev
# or: cargo tauri dev
```

### 4. Common checks

```bash
cd src-tauri && cargo test
cd src-tauri && cargo clippy
npm run lint
```

## Repository map

- `src/` — React UI, editor/player components, i18n, stores
- `src-tauri/src/managers/` — core domain logic (audio/model/transcription/editor/media/filler/history/export/project)
- `src-tauri/src/commands/` — Tauri command handlers (plus shared app/system commands)
- `.github/skills/` — Copilot skill definitions used in this repo

## Current launch focus

- Precision and reliability of transcript-driven playback/edit mapping
- Documentation alignment with the shipping Tauri architecture
- Windows-first setup reliability and repeatable build/test flow

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening PRs.

## License

MIT — see [LICENSE](LICENSE).
