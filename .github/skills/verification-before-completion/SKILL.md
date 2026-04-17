---
name: verification-before-completion
description: 'Use before claiming work is complete, fixed, or passing. Requires running verification commands and confirming output before making any success claims. Evidence before assertions, always.'
---

# Verification Before Completion

## Overview

Claiming work is complete without verification is dishonesty, not efficiency.

**Core principle:** Evidence before claims, always.

**Violating the letter of this rule is violating the spirit of this rule.**

## The Iron Law

```
NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE
```

If you haven't run the verification command in this message, you cannot claim it passes.

## The Gate Function

```
BEFORE claiming any status or expressing satisfaction:

1. IDENTIFY: What command proves this claim?
2. RUN: Execute the FULL command (fresh, complete)
3. READ: Full output, check exit code, count failures
4. VERIFY: Does output confirm the claim?
   - If NO: State actual status with evidence
   - If YES: State claim WITH evidence
5. ONLY THEN: Make the claim

Skip any step = lying, not verifying
```

## Toaster Verification Commands

Every change category has specific verification requirements:

### Rust backend changes (`src-tauri/`)

```bash
# MUST run all three — linter ≠ compiler ≠ tests
cd src-tauri && cargo check          # compilation
cd src-tauri && cargo clippy         # lint warnings
cd src-tauri && cargo test           # unit/integration tests
```

### Frontend changes (`src/`)

```bash
npm run lint                         # ESLint
npm run build                        # TypeScript compilation + Vite build
```

### Full-stack or Tauri IPC changes

```bash
# Both sides
cd src-tauri && cargo check && cargo clippy && cargo test
npm run lint && npm run build
```

### Timeline, playback, or audio-edit changes

Standard verification PLUS the acceptance gate:

```
TOASTER ACCEPTANCE GATE (non-negotiable):
1. Build and run the app
2. Load a transcript with multiple segments
3. Delete segments from the MIDDLE (not just start/end)
4. Play through the edit point — confirm NO audible remnants
5. Undo the deletion — confirm clean restoration
6. Re-delete and export — confirm exported file is clean
7. ONLY THEN claim the timeline/playback fix works
```

If you cannot run the acceptance gate (e.g., no media file available), **state that
explicitly** — do not claim the fix works based on code inspection alone.

### Timestamp or transcription changes

```
TIMESTAMP INTEGRITY CHECK:
1. cargo test (must pass)
2. Verify per-word/per-segment timing is PRESERVED, not synthesized
3. Confirm no equal-duration timestamp assignment
4. Check that backend remains the single source of truth for time mapping
```

## Common Failures

| Claim | Requires | Not Sufficient |
|-------|----------|----------------|
| Tests pass | `cargo test` output: 0 failures | Previous run, "should pass" |
| Linter clean | `cargo clippy` output: 0 errors | `cargo check` passing |
| Build succeeds | Build command: exit 0 | Linter passing |
| Bug fixed | Test reproducing original symptom passes | Code changed, assumed fixed |
| Frontend works | `npm run lint` + `npm run build` pass | "Looks correct in code" |
| Timeline fix works | Midstream deletion replay is clean | Unit tests alone |
| No regressions | Full test suite passes | Partial check |

## Red Flags — STOP

- Using "should", "probably", "seems to"
- Expressing satisfaction before verification ("Great!", "Perfect!", "Done!")
- About to commit/push/PR without verification
- Relying on partial verification (clippy but not cargo test)
- Thinking "just this once"
- **ANY wording implying success without having run verification**
- Claiming timeline/audio fixes work without replay verification
- Claiming tests pass based on a previous run (not this session)

## Rationalization Prevention

| Excuse | Reality |
|--------|---------|
| "Should work now" | RUN the verification |
| "I'm confident" | Confidence ≠ evidence |
| "Just this once" | No exceptions |
| "Linter passed" | Linter ≠ compiler ≠ tests |
| "Cargo check passed" | check ≠ clippy ≠ test |
| "Tests pass, so the fix works" | Tests ≠ acceptance gate for timeline changes |
| "I can tell from the code" | Code review ≠ execution |
| "The change is small" | Small changes cause big regressions |

## Key Patterns

**Tests:**
```
✅ [Run cargo test] [See: 34/34 pass] "All tests pass"
❌ "Should pass now" / "Looks correct"
```

**Build:**
```
✅ [Run cargo check + clippy] [See: exit 0, 0 errors] "Build and lint pass"
❌ "Clippy passed" (clippy alone doesn't prove compilation)
```

**Timeline/audio fixes:**
```
✅ [Run tests] [Run app] [Delete mid-segment] [Play through] [Confirm clean] "Midstream replay verified clean"
❌ "Tests pass, fix is complete"
```

**Requirements:**
```
✅ Re-read plan → Create checklist → Verify each → Report gaps or completion
❌ "Tests pass, phase complete"
```

## When To Apply

**ALWAYS before:**
- ANY variation of success/completion claims
- ANY expression of satisfaction about work state
- Committing, PR creation, task completion
- Moving to next task
- Claiming a bug is fixed
- Claiming no regressions

**The Bottom Line:**
Run the command. Read the output. THEN claim the result. No shortcuts.
