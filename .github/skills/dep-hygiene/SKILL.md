---
name: dep-hygiene
description: 'Use before adding a dependency, after removing a module, and on any PR that claims "dead code cleanup". Enforces cargo machete / depcheck / knip gates so unused deps and orphaned modules do not accumulate.'
---

# Dependency Hygiene

## Overview

Toaster accumulated a large block of Handy-era dependencies (rdev, enigo, rodio, cpal, vad-rs, handy-keys, tauri-nspanel, gtk-layer-shell, …) that survived long after their consuming modules became dead. Unused deps inflate build time, attack surface, and cognitive load.

**Core principle:** A dependency only exists because at least one live file imports it.

## The Iron Law

```
DELETING A MODULE = CHECKING ITS DEPS FOR ORPHAN STATUS,
IN THE SAME PR.
```

## Tools

### Rust (`src-tauri/`)

- `cargo machete` — lists crates declared in `Cargo.toml` that no `use` references. Install: `cargo install cargo-machete`.
- `cargo tree -d` — shows duplicate versions of the same crate pulled in transitively.
- `cargo udeps --all-targets` (nightly) — stricter than machete; catches unused via `cfg` gates. Optional.

### Frontend (`/`)

- `npx knip` — finds unused files, exports, and dependencies in the TS/React tree.
- `npx depcheck` — simpler alternative focused on `package.json`.

## Gate Function

**Adding a dependency:**

```
1. JUSTIFY in the PR description: which file imports it, for what use.
2. PREFER an existing dep that already does the job.
3. AVOID deps that only serve a single dictation-era feature (see
   handy-legacy-pruning skill).
4. After adding: re-run cargo check / npm run build to confirm the bloat is real.
```

**Removing a module:**

```
1. rg "<removed-module-symbol>" src src-tauri/src   # confirm no stragglers
2. cargo machete                                    # lists newly-orphaned crates
3. Remove each orphaned crate from Cargo.toml
4. cargo check && cargo clippy && cargo test
5. For frontend: npx knip
6. Remove orphaned npm packages from package.json
7. npm install && npm run lint && npm run build
```

**Reviewing a "cleanup" PR:**

```
1. DEMAND cargo machete output in the PR body
2. DEMAND knip/depcheck output if the frontend is touched
3. If Cargo.toml / package.json is not reduced, ask why
```

## High-Value Targets (as of current audit)

Rust crates with Handy-era-only consumers — candidates for removal once their modules are deleted:

- `rdev`, `enigo`, `handy-keys` — keyboard synth + hotkey capture (dictation)
- `rodio` — recording start/stop sounds
- `cpal` — live microphone capture
- `vad-rs` — voice activity detection for recorder
- `tauri-plugin-global-shortcut`, `tauri-plugin-autostart`, `tauri-plugin-single-instance`, `tauri-plugin-macos-permissions`
- `tauri-nspanel`, `gtk-layer-shell`, `gtk` — overlay window implementations
- `winreg` (Windows-only) — used by keyboard-impl detection only

npm packages:

- `@tauri-apps/plugin-autostart` — dictation-only consumer
- `@tauri-apps/plugin-global-shortcut` — review for editor use
- `tauri-plugin-macos-permissions-api` — STILL LIVE (onboarding)

Patched dependencies:

- `[patch.crates-io] tauri-runtime* = cjpais/tauri handy-2.10.2` — re-test upstream after overlay/global-shortcut/nspanel removal.

## Red Flags — STOP

- PR removes 1000 LOC of code but 0 lines of `Cargo.toml` / `package.json`
- Adding a dep "just for this one small helper" you could inline
- Adding a dep that duplicates capability already provided by `tauri-*` plugins
- Pinning a dep to a fork without a link to the upstream issue
- Silencing `cargo machete` with an allow-list without justification in the PR

## When To Apply

- Every PR that adds or removes a dependency
- Every PR that deletes a module
- Every PR with "cleanup" / "prune" / "dead code" in the title
- Before releasing — run `cargo machete` and `npx knip` as a final gate
