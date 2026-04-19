---
name: dep-hygiene
description: 'Use before adding a Rust crate or npm package, after deleting a module, and on any PR that claims "dead code cleanup". Enforces cargo machete / knip / depcheck gates so orphaned Handy-era dependencies do not accumulate. Toaster-specific; runs alongside superpowers:systematic-debugging and superpowers:receiving-code-review.'
---

# Dep Hygiene

Toaster accumulated a large block of Handy-era dependencies (rdev, enigo,
rodio, cpal, vad-rs, handy-keys, tauri-nspanel, gtk-layer-shell, …) that
survived long after their consuming modules became dead. This skill keeps
new code from recreating that drift.

```
DELETING A MODULE = CHECKING ITS DEPS FOR ORPHAN STATUS,
IN THE SAME PR.
```

A dependency only exists because at least one live file imports it.

## Tools

- **Rust:** `cargo machete` (declared-but-unused), `cargo tree -d` (duplicate
  versions). Optional: `cargo udeps --all-targets` (nightly).
- **Frontend:** `npx knip` (unused files, exports, deps) or `npx depcheck`.

## Adding a dependency

1. Justify in the PR body: which file imports it and why.
2. Prefer an existing dep that already does the job.
3. Avoid deps serving only a dictation-era feature — see `handy-legacy-pruning`.
4. After adding, rerun `cargo check` / `npm run build`.

## Removing a module

```
1. rg "<removed-symbol>" src src-tauri/src     # no stragglers
2. cargo machete                                # orphaned crates
3. Remove each from Cargo.toml
4. cargo check && cargo clippy && cargo test
5. npx knip                                     # frontend if touched
6. Remove orphaned npm packages
7. npm install && npm run lint && npm run build
```

## Reviewing a "cleanup" PR

- Demand `cargo machete` output in the PR body.
- Demand `knip` / `depcheck` output if frontend is touched.
- If `Cargo.toml` / `package.json` isn't reduced, ask why.

## Red flags

- PR removes 1000 LOC of code but 0 lines of `Cargo.toml` / `package.json`.
- Adding a dep "for one small helper" you could inline.
- Adding a dep that duplicates a `tauri-*` plugin.
- Silencing `cargo machete` with an allow-list without justification.

## High-value removal targets (2026-04 audit)

Rust crates with Handy-era-only consumers: `rdev`, `enigo`, `handy-keys`,
`rodio`, `cpal`, `tauri-plugin-global-shortcut`,
`tauri-plugin-autostart`, `tauri-plugin-single-instance`,
`tauri-nspanel`, `gtk-layer-shell`, `gtk`,
`winreg`.

Note: `vad-rs` was previously on this list but is **not** used; the
reintroduced VAD path (R-002 / R-003 / R-004) speaks ORT directly via the
existing `ort = 2.0.0-rc.12` pulled in by `transcribe-rs`. If a future
audit re-surfaces `vad-rs` as a candidate, verify first that
`src-tauri/src/audio_toolkit/vad/` has not been re-wired to it — the
BLUEPRINT mandates one ONNX runtime, not two.

npm: `@tauri-apps/plugin-autostart` (dictation-only consumer),
`@tauri-apps/plugin-global-shortcut` (review for editor use).

Removed (2026-04 prune batch): `tauri-plugin-macos-permissions` +
`tauri-plugin-macos-permissions-api` — Toaster is a file-based transcript
editor and does not record audio or inject keystrokes, so the macOS
Accessibility + Microphone permission onboarding was dead code.

Patched deps: re-test upstream `tauri-runtime*` after overlay /
global-shortcut / nspanel removal.

## Related skills

- `handy-legacy-pruning` — identifies which modules are dictation-only and
  can be deleted, orphaning their deps.
- `superpowers:systematic-debugging` — if a removal breaks something,
  root-cause before rolling forward.
- `superpowers:verification-before-completion` — cleanup PRs must show
  `cargo machete` / `knip` output as evidence.
