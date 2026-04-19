# Cached backend outputs for multi-backend parity eval

Layout: `backend_outputs/<backend>/<fixture_stem>.result.json`

Each file is a `NormalizedTranscriptionResult` (see
`src-tauri/src/managers/transcription/adapter.rs`) captured from a real
run of `<backend>` against `<fixture_stem>.wav`. The parity runner
(`scripts/eval/eval-multi-backend-parity.ps1`) consumes these to compute
per-backend boundary error vs the oracle and cross-backend parity.

If no result file exists for a (backend, fixture) pair, the runner logs
a `skip` with the reason. In `-StrictMode` skip promotes to fail.

Regeneration: the eval-harness-runner agent or a developer with the
backend installed runs the app, transcribes the fixture, serializes the
adapter output here. Do NOT hand-author these files — they must come
from the real adapter path, otherwise the gate is not measuring the
adapter.

Schema (subset):

```json
{
  "words": [
    { "text": "hello", "start_us": 0, "end_us": 420000, "confidence": null }
  ],
  "language": "en-US",
  "word_timestamps_authoritative": true,
  "input_sample_rate_hz": 16000
}
```
