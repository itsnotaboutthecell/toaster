---
name: test-driven-development
description: 'Use when implementing any feature or bugfix, before writing implementation code. Write the test first, watch it fail, then write minimal code to pass.'
---

# Test-Driven Development (TDD)

## Overview

Write the test first. Watch it fail. Write minimal code to pass.

**Core principle:** If you didn't watch the test fail, you don't know if it tests the right thing.

## The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Write code before the test? Delete it. Start over.

**No exceptions:**
- Don't keep it as "reference"
- Don't "adapt" it while writing tests
- Delete means delete

## When to Use

**Always:**
- New features
- Bug fixes
- Refactoring
- Behavior changes

**Exceptions (ask the user):**
- Throwaway prototypes
- Configuration-only changes
- Pure UI styling changes (Tailwind classes)

## Red-Green-Refactor

### RED — Write Failing Test

Write one minimal test showing what should happen.

**Rust (backend):**
```rust
#[test]
fn test_keeps_segments_around_deletion() {
    let segments = vec![/* setup */];
    let result = compute_keep_segments(&segments, &deleted_range);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].end_us, expected_boundary);
}
```

**TypeScript (frontend):**
```typescript
test('formats timestamp in microseconds correctly', () => {
  expect(formatTimestamp(1500000)).toBe('00:01.500');
});
```

**Requirements:**
- One behavior per test
- Clear descriptive name
- Real code (no mocks unless unavoidable)

### Verify RED — Watch It Fail

**MANDATORY. Never skip.**

```bash
# Rust
cd src-tauri && cargo test test_name -- --nocapture

# Frontend
npm test -- --run path/to/test
```

Confirm:
- Test FAILS (not errors from typos)
- Failure message is what you expect
- Fails because the feature is missing

**Test passes?** You're testing existing behavior. Fix the test.
**Test errors?** Fix the error, re-run until it fails correctly.

### GREEN — Minimal Code

Write the simplest code to pass the test. Don't add features, refactor, or "improve" beyond the test.

### Verify GREEN — Watch It Pass

**MANDATORY.**

```bash
cd src-tauri && cargo test     # ALL Rust tests
npm run lint                   # Frontend lint
```

Confirm:
- Your test passes
- ALL other tests still pass
- No new warnings

**Test fails?** Fix code, not the test.
**Other tests fail?** Fix now — don't defer.

### REFACTOR — Clean Up

After green only:
- Remove duplication
- Improve names
- Extract helpers

Keep all tests green. Don't add behavior during refactoring.

### Repeat

Next failing test for next behavior.

## Toaster Testing Commands

```bash
# Run all Rust tests
cd src-tauri && cargo test

# Run specific test with output
cd src-tauri && cargo test test_name -- --nocapture

# Run tests matching a pattern
cd src-tauri && cargo test filter_filler -- --nocapture

# Frontend lint (no test framework set up yet for unit tests)
npm run lint

# E2E tests (Playwright)
npx playwright test
```

## Good Tests

| Quality | Good | Bad |
|---------|------|-----|
| **Minimal** | One thing. "and" in name? Split it. | `test_validates_and_exports_and_plays` |
| **Clear** | Name describes behavior | `test1`, `it_works` |
| **Shows intent** | Demonstrates desired API | Obscures what code should do |
| **Uses real values** | Actual timestamps in microseconds | Magic numbers without context |

## Bug Fix Pattern

**Bug:** Midstream deletion leaves audible remnant

1. **RED:** Write test reproducing the exact scenario
   ```rust
   #[test]
   fn test_midstream_deletion_produces_clean_boundaries() {
       // Setup: segments spanning a deletion in the middle
       // Assert: keep-segments have correct boundaries with no gap/overlap
   }
   ```

2. **Verify RED:** `cargo test test_midstream -- --nocapture` → FAILS

3. **GREEN:** Fix the boundary calculation in the manager

4. **Verify GREEN:** `cargo test` → ALL pass

5. **REFACTOR:** Clean up if needed

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "Too simple to test" | Simple code breaks. Test takes 30 seconds. |
| "I'll test after" | Tests passing immediately prove nothing. |
| "Need to explore first" | Fine. Throw away exploration, start with TDD. |
| "Test is hard to write" | Hard to test = hard to use. Listen to the test. |
| "Existing code has no tests" | You're improving it. Add tests for what you touch. |
| "The change is trivial" | Trivial changes cause non-trivial regressions. |

## Red Flags — STOP and Start Over

- Code written before test
- Test passes immediately (never saw it fail)
- Can't explain why the test failed
- Tests added "later"
- Rationalizing "just this once"
- "It's about spirit not ritual"

**All of these mean: Delete code. Start over with TDD.**

## Verification Checklist

Before marking work complete:

- [ ] Every new function/method has a test
- [ ] Watched each test fail before implementing
- [ ] Each test failed for expected reason
- [ ] Wrote minimal code to pass each test
- [ ] All tests pass (`cargo test`, `npm run lint`)
- [ ] Tests use real code (mocks only if unavoidable)
- [ ] Edge cases and error paths covered
