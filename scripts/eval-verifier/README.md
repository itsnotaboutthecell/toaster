# eval-verifier

Offline LLM-as-a-Verifier harness for Toaster accuracy gates.

Adapted from [llm-as-a-verifier/llm-as-a-verifier](https://github.com/llm-as-a-verifier/llm-as-a-verifier)
(Terminal-Bench 2 / SWE-Bench Verified SOTA trajectory reward model). The
technique is adopted; the upstream Gemini runtime dependency is NOT.

## Why this exists

Toaster has multi-backend ASR outputs (`whisper`, `parakeet`, ...) for the
same audio. When backends disagree, existing eval gates tell you *each*
backend's numeric error vs the oracle, but they do not rank them. This
harness does Best-of-N ranking with:

- **Fine-grained scoring** (20-letter scale A..T, logprob expectation).
- **Repeated verifications** (default K=4).
- **Criteria decomposition** (3 orthogonal rubrics).
- **Round-robin tournament** over all backend pairs.

It is complementary to `scripts/eval-multi-backend-parity.ps1`, not a
replacement. The numeric parity eval is the acceptance gate; this harness
produces a calibrated *preference* between backends for the same audio.

## Non-goals

- **NOT a runtime feature.** This harness lives in `scripts/` and runs on
  developer / CI machines only. Toaster's product is local-only — see
  AGENTS.md "Local-only inference".
- **NOT a replacement for objective gates.** Audio-boundary seam gates
  (xcorr, HF-burst energy, sample discontinuity) stay in
  `scripts/eval-audio-boundary.ps1`; LLM judgement adds noise there.

## Usage

Mock backend (default, no network, deterministic):

```powershell
.\scripts\eval-verifier.ps1
```

Local OpenAI-compatible server (llama.cpp, vLLM, Ollama's OpenAI shim):

```powershell
.\scripts\eval-verifier.ps1 -Backend openai-compat -BaseUrl http://127.0.0.1:8080/v1 -Model my-local-model
```

Gemini (CI / upstream-parity development ONLY):

```powershell
$env:GEMINI_API_KEY = "<key>"
.\scripts\eval-verifier.ps1 -Backend gemini
```

Output lands under `eval/output/verifier-parity/<stamp>/` as `report.json` and
`report.md`. Exit code is non-zero if any fixture fails the pass policy:

- winning trial's `p95_err_us` exceeds `--p95-floor-us` (default 40 000 µs,
  matches G2 gate in `eval-multi-backend-parity`), or
- winning trial has `equal_duration_fraction >= 0.95` (synthesized timings).

## Runners

Four runners ship today. Each takes the same backend flags and emits a
`report.json` + `report.md` under `eval/output/verifier-<runner>/<stamp>/`.

| Wrapper | Fixtures | Criteria |
| --- | --- | --- |
| `scripts/eval-verifier.ps1` | `src-tauri/tests/fixtures/parity/` | ASR transcription + timing + authoritative-claim honesty |
| `scripts/eval-verifier-cleanup.ps1` | `src-tauri/tests/fixtures/cleanup/` | Filler removal recall, content preservation, timing monotonicity, deleted-region audibility |
| `scripts/eval-verifier-captions.ps1` | `src-tauri/tests/fixtures/captions/` | Readability, punctuation respect, line-length balance, timing coverage |
| `scripts/eval-verifier-disfluency.ps1` | `src-tauri/tests/fixtures/disfluency/` | Group collapse completeness, audio-aware survivor clarity, audio-aware cut placement cleanliness, pacing agreement (vs. human oracle), timing monotonicity |

The cleanup and disfluency runners both pick up ``audio_path`` from
their fixture JSON (resolved relative to the fixture file) and use
``audio_features.py`` — a stdlib-only WAV reader — to compute the mean
silence ratio of each candidate's deleted regions, the articulation
score (peak + RMS + silence) of each repetition survivor, and the
silence ratio of a 40 ms window centered on every splice seam.
Candidates that "delete" silence instead of audible speech, or that
collapse a "the the best best" group by keeping the mumbled take, rank
strictly below candidates that keep the clearer take — which is the
behavior the repo's fixtures were constructed to prove.

## Criteria

See `criteria_*.py`. Each criterion ID is load-bearing — `backends._mock_score`
dispatches on it. Adding a criterion means adding a mock-scoring branch.

| ID | Rubric | Penalizes |
| --- | --- | --- |
| `transcription_fidelity` | parity | Substitutions / insertions / deletions vs oracle |
| `word_timing_fidelity` | parity | High median/p95 error, equal-duration synthesis |
| `authoritative_honesty` | parity | `authoritative=true` while p95 blows past the gate |
| `filler_removal_recall` | cleanup | Leaving non-quoted fillers in the keep set |
| `content_preservation` | cleanup | Deleting content words or quoted fillers |
| `deleted_region_audible` | cleanup | Deleting silence instead of audible speech (requires audio) |
| `group_collapse_completeness` | disfluency | Groups left with 0 or >1 survivors |
| `survivor_clarity` | disfluency | Keeping the mumbled repetition instead of the clearer take (requires audio) |
| `cut_placement_cleanliness` | disfluency | Cut seams that land inside voiced speech (requires audio) |
| `pacing_agreement` | disfluency | Disagreement with the human oracle's keep/delete labels on non-filler, non-repetition words (only active on fixtures that carry `human_label`) |
| `timing_monotonicity` | disfluency / cleanup | Non-monotonic / overlapping kept segments |
| `readability` | captions | Line length outside ~20-42 chars, >7 words/line |
| `punctuation_respect` | captions | Splits inside quoted spans, breaks mid-clause |
| `line_length_balance` | captions | High CV across line character counts |
| `timing_coverage` | captions | Missing or duplicated oracle words across lines |

## Backends

- `mock` — deterministic analytical scorer. Reads numeric hints from the
  formatted trajectory string and scores without any LLM call. This is what
  CI uses. It is only as good as the criterion's numeric rubric allows —
  cross-axis reasoning (e.g. "backend claims authoritative but the audio
  shows silence overrun") cannot emerge from it.
- `openai-compat` — any HTTP endpoint speaking OpenAI `/v1/chat/completions`
  with `top_logprobs`. Preferred when a real model is warranted; keep it on
  `127.0.0.1` to preserve Toaster's local-first ethos.
- `gemini` — upstream-parity path via `google-genai` + Vertex. Requires
  `GEMINI_API_KEY` or `VERTEX_API_KEY`. **Never ship in the product.**

## Dependencies

Stdlib only for `mock` and `openai-compat`. Gemini backend additionally
requires `google-genai` — install with `pip install google-genai`.

## Layout

```
scripts/eval-verifier/
  README.md
  verifier_core.py        # scoring + tournament (generic)
  backends.py             # mock / openai-compat / gemini
  audio_features.py       # stdlib WAV window features (RMS, peak, silence ratio)
  criteria_parity.py      # ASR parity rubrics + Trial builder
  criteria_cleanup.py     # filler-removal rubrics + Trial builder
  criteria_captions.py    # caption-grouping rubrics + Trial builder
  criteria_disfluency.py  # repetition-collapse rubrics + Trial builder
  generate_disfluency_fixtures.py  # stdlib WAV synth for disfluency fixtures
  run_parity.py           # entrypoint for eval-verifier.ps1
  run_cleanup.py          # entrypoint for eval-verifier-cleanup.ps1
  run_captions.py         # entrypoint for eval-verifier-captions.ps1
  run_disfluency.py       # entrypoint for eval-verifier-disfluency.ps1
  tests/
    test_smoke.py         # parity smoke test
    test_cleanup_smoke.py # cleanup smoke test
    test_captions_smoke.py# captions smoke test
    test_disfluency_smoke.py # disfluency smoke test
    test_audio_features.py# WAV feature unit tests
```

## Smoke tests

```powershell
python -m unittest discover -s scripts\eval-verifier\tests -p 'test_*.py' -v
```

Fourteen tests run end-to-end against the mock backend plus the
shipping fixtures. They assert the tournament prefers Parakeet over
Whisper, picks the perfect cleanup candidate over both over- and
under-deletion failure modes (including the quote-violation failure
mode specific to Toaster), picks the balanced caption grouping over
single-line / one-word-per-line / quote-splitting / word-dropping
failure modes, and — for disfluency — picks the candidate that keeps
the *clear* take of a repeated word over one that keeps the mumbled
take, strictly ordering them by audio articulation even when group
collapse stats are identical.
