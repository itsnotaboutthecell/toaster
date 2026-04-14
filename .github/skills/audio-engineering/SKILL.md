# Audio Engineering Skill

> **⚠️ FORWARD-LOOKING**: The implementation targets referenced below (`plugins/whisper-transcribe/`, `frontend/PlaybackEngine.cpp`) **do not exist yet**. This skill documents the *intended* approach for when sample-level boundary refinement is built (see PRD.md Phase 3). Do not reference these files as if they exist today.

Use this skill when transcript word boundaries, deletion joins, or perceived cut quality are not accurate enough.

## What this skill does
- Evaluates waveform boundaries at sample precision
- Synthesizes controlled audio for repeatable boundary tests
- Translates audio-engineering findings into concrete C/C++ implementation tasks

## Trigger phrases
- "word boundary precision"
- "cut seam"
- "audio click/pop at delete"
- "waveform alignment"
- "speech boundary detection"
- "new release items deletes both words"

## Engineering objective
Increase boundary precision from coarse token timestamps (10 ms) to sample-driven boundaries.
At 16 kHz, one sample is 62.5 microseconds, which is >100x finer than 10 ms.

## Workflow

### 1. Measure
1. Decode source to mono float PCM at 16 kHz.
2. For each adjacent word boundary, inspect a local window (for example ±100 ms).
3. Compute local short-window energy (for example mean abs amplitude over 2.5 ms).
4. Select the lowest-energy valley near the coarse boundary.
5. Snap to nearest near-zero crossing (for example ±1 ms search).

### 2. Refine timestamps
1. Convert candidate boundary sample to microseconds.
2. Enforce constraints:
- Word i end >= word i start + min_word_us
- Word i+1 start <= word i+1 end - min_word_us
3. Write boundary to both sides:
- prev.end_us = boundary_us
- next.start_us = boundary_us

### 3. Validate quality
1. No overlaps: end_us >= start_us for all words.
2. Monotonic sequence: word[i+1].start_us >= word[i].end_us.
3. Delete-only one word in close pairs (for example "new release") does not remove neighbor.
4. No audible clicks at delete joins in preview and export.

### 4. Translate findings for software engineers
Use this template:
- Symptom: what user hears/sees
- Signal evidence: valley index, local energy stats, zero-crossing offset
- Root cause: coarse timestamp collision / boundary drift / stale seek state
- Code action: exact file and function to update
- Verification: tests and manual playback case

## Reference implementation targets in Toaster (planned, not yet created)
- Boundary generation: plugins/whisper-transcribe/whisper-transcribe.c (Phase 3)
- Runtime join behavior: frontend/PlaybackEngine.cpp (Phase 3)
- Timeline mapping: libtoaster/toaster.c (existing — undo/redo and split word)

## Synthesis recipes (controlled test assets)

### A. Boundary discrimination test
Generate two spoken words separated by short silence (40 ms, 20 ms, 10 ms, 5 ms).
Expected: system finds distinct boundaries and deleting one does not remove both.

### B. Hard-cut seam test
Create two tones/noise bursts with a synthetic boundary.
Expected: join point lands near a zero crossing; no click/pop.

### C. Stress test
Randomize boundaries and speech-like envelopes over 10,000 cases.
Expected: no overlap, no negative durations, stable map/unmap behavior.

## Metrics to report
- Mean absolute boundary error (microseconds)
- 95th percentile boundary error
- Adjacent-word collision rate
- Click/pop incidence at joins
- Preview/export agreement at boundary timestamp

## Done criteria
- Boundary precision consistently sub-millisecond on test corpus
- Adjacent-word deletion false-positive rate near zero
- Preview and export boundaries match within 1 frame video / 1 audio buffer window
- Regression tests cover partial-overlap, near-adjacent words, and pause shortening edge cases
