# Area categorization: chapter markers

Check all areas this feature touches:

- [ ] Frontend (React/TS)            — no UI in this bundle.
- [x] Backend (Rust managers)        — new `managers::export::chapters`
                                       module; new helpers in
                                       `commands::waveform`.
- [ ] Audio path                     — no DSP changes; uses existing
                                       keep-segments + stretch map.
- [x] Transcription adapter          — read-only dependency on
                                       paragraph grouping surfaced by
                                       `managers::transcription`.
- [x] Export pipeline                — adds ffmetadata input, sidecar
                                       write, `-map_metadata` arg.
- [ ] Captions / UI strings (i18n)   — no user-visible strings.
- [ ] Settings UI                    — opt-out toggle is out of scope.
- [x] Evals                          — new fixture eval scripts under
                                       `scripts/eval/` (stubs committed
                                       with the plan; real
                                       implementations land with the
                                       feature).
