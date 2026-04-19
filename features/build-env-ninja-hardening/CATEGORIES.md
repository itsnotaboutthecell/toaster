# Area categorization: build env Ninja hardening

Check all areas this feature touches:

- [ ] Frontend (React/TS)
- [ ] Backend (Rust managers)
- [ ] Audio path
- [ ] Transcription adapter
- [ ] Export pipeline
- [ ] Captions / UI strings (i18n)
- [ ] Settings UI
- [ ] Evals
- [x] Build / toolchain (`scripts/setup-env.ps1`,
      `scripts/launch-toaster-monitored.ps1`, new
      `scripts/gate/check-cmake-ninja-env.ps1`)
- [x] Docs (`docs/build.md` "Build environment gotchas")
