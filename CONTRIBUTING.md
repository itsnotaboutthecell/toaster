# Contributing to Toaster

Thanks for contributing.

## Before you start

1. Search existing issues/PRs to avoid duplicate work.
2. If your change affects playback precision or timeline mapping, include reproduction details and expected behavior.
3. Keep scope focused; separate unrelated fixes into separate PRs.

## Development setup

See [BUILD.md](BUILD.md) for full setup.

Quick path:

```bash
bun install --frozen-lockfile
```

Windows (same shell before Cargo/Tauri commands):

```powershell
.\scripts\setup-env.ps1
```

Run:

```bash
npm run tauri dev
# or: cargo tauri dev
```

## Validation before opening PR

```bash
cd src-tauri && cargo test
cd src-tauri && cargo clippy
npm run lint
```

## Project conventions

- Backend managers own business logic; frontend should call Tauri commands.
- Timestamps are microseconds in backend word/timeline structures.
- Keep-segment/time mapping in backend is authoritative for edit-time/source-time behavior.
- Never switch video rendering source to audio preview source in playback code; use separate synced audio track/path.
- All UI text should use i18n keys.

## Pull request guidance

- Use conventional commit prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`).
- Describe user-visible behavior changes clearly.
- Include screenshots/video for UI changes when useful.
- For precision/playback/export changes, include the scenario used to verify midstream edits and delete/undo cycles.

## Documentation updates

If behavior or workflows changed, update relevant docs in the same PR:

- `README.md`
- `BUILD.md`
- `AGENTS.md` / `.github/copilot-instructions.md` when agent behavior guidance changed
