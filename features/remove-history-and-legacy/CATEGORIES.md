# Area categorization: remove history and legacy

Areas this feature touches:

- [x] Frontend (React/TS) — Sidebar, settings panels, settingsStore
- [x] Backend (Rust managers) — managers::history, settings types
- [ ] Audio path — untouched
- [ ] Transcription adapter — comment refs only
- [ ] Export pipeline — untouched
- [x] Captions / UI strings (i18n) — full sweep across 20 locales
- [x] Settings UI — Advanced panel deletion + relocations
- [ ] Evals — none new; existing precision/boundary evals must
      continue to pass after deletion (regression-only)

Cross-cutting skills invoked:

- handy-legacy-pruning — verified the classic Handy file list is
  already absent from the tree.
- dep-hygiene — orphaned crate / npm package sweep after deletion.
- i18n-pruning — locale parity gate after key removal.
- canonical-instructions — no instruction-file changes (AGENTS.md
  layout still mentions `managers/history`; will need a one-line
  update in AGENTS.md as part of the cleanup).
