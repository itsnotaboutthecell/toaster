# Commands

Authoritative structured data lives in [`.github/registry/commands.json`](../.github/registry/commands.json). This document renders the current contents for human reading.

Query subsets via the reader CLI:

```bash
bun scripts/registry/reader.ts commands --tier fast    # inner loop
bun scripts/registry/reader.ts commands --tier full    # milestone sweep
bun scripts/registry/reader.ts commands --tier live    # live app + evals
bun scripts/registry/reader.ts render commands         # markdown render
```

## Fast inner loop (iteration)

```bash
# Backend (run from src-tauri/)
cargo check -p toaster --lib                         # type-check one crate (~60s cold, <10s warm)
cargo clippy -p toaster --lib                        # lint one crate
cargo test -p toaster --lib <test_name>              # single test
cargo test -p toaster --lib <module>::               # one module's tests
cargo test test_filter_filler_words -- --nocapture   # single test with stdout

# Frontend
npm run lint -- src/components/editor/               # scoped lint
bun run check:file-sizes                             # 800-line cap gate
bun scripts/check-translations.ts                    # i18n parity (20 locales)
```

## Full sweep (once per milestone)

```bash
bun install --frozen-lockfile
cd src-tauri && cargo check                          # 2–10 min cold
cd src-tauri && cargo clippy                         # 2–10 min cold
cd src-tauri && cargo test                           # full unit + integration
npm run build                                        # vite production build
npm run lint                                         # full eslint pass
```

## Live app + evals (verification)

```bash
# Full dev app — first run compiles ~689 crates
cargo tauri dev

# Monitored live-app verification (required for audio / caption / preview / export fixes)
pwsh scripts/launch-toaster-monitored.ps1 -Duration 5m

# Fixture-based eval harness
pwsh scripts/eval/eval-edit-quality.ps1              # precision eval
pwsh scripts/eval/eval-audio-boundary.ps1            # splice seam quality
pwsh scripts/eval/eval-multi-backend-parity.ps1      # ASR backend parity
```

See [`src-tauri/AGENTS.md`](../src-tauri/AGENTS.md) for Windows build environment, cargo runtime expectations, and DLL pitfalls (incl. `0xc0000139` / 0-byte DirectML.dll recovery).
