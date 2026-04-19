# Area categorization: readme launch pass

Check all areas this feature touches:

- [ ] Frontend (React/TS)
- [ ] Backend (Rust managers)
- [ ] Audio path
- [ ] Transcription adapter
- [ ] Export pipeline
- [ ] Captions / UI strings (i18n)
- [ ] Settings UI
- [ ] Evals
- [x] Documentation (top-level `README.md` only; `CONTRIBUTING.md` and
      `docs/build.md` remain canonical and are linked, not duplicated)

Notes:

- Zero production-code surface. Everything happens in Markdown.
- README is English-only (per
  `features/readme-launch-pass/REQUEST.md` § 5); no i18n impact.
- No new top-level docs; no changes to `docs/build.md` content in
  this bundle (the README only cites it).