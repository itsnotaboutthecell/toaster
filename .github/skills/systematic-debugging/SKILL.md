---
name: systematic-debugging
description: 'Use when encountering any bug, test failure, or unexpected behavior, before proposing fixes. Requires root cause investigation before attempting solutions.'
---

# Systematic Debugging

## Overview

Random fixes waste time and create new bugs. Quick patches mask underlying issues.

**Core principle:** ALWAYS find root cause before attempting fixes. Symptom fixes are failure.

## The Iron Law

```
NO FIXES WITHOUT ROOT CAUSE INVESTIGATION FIRST
```

If you haven't completed Phase 1, you cannot propose fixes.

## When to Use

Use for ANY technical issue:
- Test failures
- Bugs in production
- Unexpected behavior
- Build failures
- Audio/timeline regressions
- Tauri IPC mismatches
- Frontend rendering issues

**Use this ESPECIALLY when:**
- Under time pressure
- "Just one quick fix" seems obvious
- You've already tried multiple fixes
- Previous fix didn't work
- You don't fully understand the issue

## The Four Phases

Complete each phase before proceeding to the next.

### Phase 1: Root Cause Investigation

**BEFORE attempting ANY fix:**

1. **Read Error Messages Carefully**
   - Don't skip past errors or warnings
   - Read stack traces completely (Rust backtraces, browser console, Tauri logs)
   - Note line numbers, file paths, error codes

2. **Reproduce Consistently**
   - Can you trigger it reliably?
   - What are the exact steps?
   - If not reproducible → gather more data, don't guess

3. **Check Recent Changes**
   - What changed that could cause this?
   - `git --no-pager diff` and recent commits
   - New dependencies, config changes

4. **Trace Across Toaster's Component Boundaries**

   Toaster has clear component boundaries. When debugging, trace across them:

   ```
   Frontend (React/TS)
     ↕ Tauri invoke() / listen()
   Commands (src-tauri/src/commands/)
     ↕ function calls
   Managers (src-tauri/src/managers/)
     ↕ function calls
   Audio Toolkit (src-tauri/src/audio_toolkit/)
   ```

   **For each boundary crossing:**
   - Log/inspect what data enters the component
   - Log/inspect what data exits the component
   - Verify types and values match expectations

   **Example — timeline mismatch:**
   ```rust
   // Manager layer: what timestamps does it produce?
   eprintln!("keep_segments: {:?}", segments);

   // Command layer: what does the IPC return?
   eprintln!("command returning: {:?}", result);
   ```
   ```typescript
   // Frontend: what did it receive?
   console.log("received from backend:", data);
   ```

   Run once to gather evidence showing WHERE it breaks, THEN investigate.

5. **Trace Data Flow**
   - Where does the bad value originate?
   - What called this with the bad value?
   - Keep tracing upstream until you find the source
   - Fix at source, not at symptom

### Phase 2: Pattern Analysis

1. **Find Working Examples**
   - Locate similar working code in the same codebase
   - What works that's similar to what's broken?

2. **Compare Against References**
   - Read reference implementation COMPLETELY (don't skim)
   - Understand the pattern fully before applying

3. **Identify Differences**
   - What's different between working and broken?
   - List every difference, however small

4. **Respect Architecture Boundaries**
   - Backend managers own business logic
   - Frontend renders state, doesn't compute timeline
   - Backend keep-segment/time-mapping is the single source of truth
   - If your fix puts logic in the wrong layer, STOP

### Phase 3: Hypothesis and Testing

1. **Form Single Hypothesis**
   - State clearly: "I think X is the root cause because Y"
   - Be specific, not vague

2. **Test Minimally**
   - Make the SMALLEST possible change
   - One variable at a time
   - Don't fix multiple things at once

3. **Verify Before Continuing**
   - Did it work? Yes → Phase 4
   - Didn't work? Form NEW hypothesis
   - DON'T add more fixes on top

4. **When You Don't Know**
   - Say "I don't understand X"
   - Don't pretend to know
   - Ask for help

### Phase 4: Implementation

1. **Create Failing Test Case**
   - Simplest possible reproduction
   - For Rust: add a `#[test]` in the relevant manager/module
   - For frontend: add a test or reproduce in Playwright
   - MUST exist before fixing

2. **Implement Single Fix**
   - Address the ROOT CAUSE identified
   - ONE change at a time
   - No "while I'm here" improvements

3. **Verify Fix**
   - `cargo test` passes
   - `cargo clippy` clean
   - `npm run lint` clean (if frontend touched)
   - For timeline/audio: run acceptance gate (see verification-before-completion skill)

4. **If Fix Doesn't Work**
   - STOP
   - Count: How many fixes have you tried?
   - If < 3: Return to Phase 1 with new information
   - **If ≥ 3: STOP and question the architecture**
   - DON'T attempt Fix #4 without discussing with the user

5. **If 3+ Fixes Failed: Question Architecture**

   Pattern indicating architectural problem:
   - Each fix reveals new coupling in a different place
   - Fixes require "massive refactoring"
   - Each fix creates new symptoms elsewhere

   **STOP and discuss with the user before attempting more fixes.**

## Toaster-Specific Debugging Patterns

### Audio/Timeline Issues
```
1. Check backend keep-segment logic first (managers/editor.rs)
2. Verify timestamp units (must be microseconds, matching AV_TIME_BASE)
3. Check if frontend is creating independent timeline logic (it shouldn't)
4. Trace: editor manager → command handler → frontend store → player component
```

### Tauri IPC Issues
```
1. Check command registration in lib.rs collect_commands!
2. Verify serde serialization matches between Rust struct and TS type
3. Check that invoke() call name matches #[tauri::command] function name
4. Look for async/sync mismatch
```

### Build Failures (Windows)
```
1. Run scripts\setup-env.ps1 in current shell
2. Check MSVC vs GNU target (must be MSVC)
3. Stop running toaster processes before rebuild
4. Check LIBCLANG_PATH and VULKAN_SDK
```

## Red Flags — STOP and Follow Process

If you catch yourself:
- "Quick fix for now, investigate later"
- "Just try changing X and see if it works"
- "It's probably X, let me fix that"
- "I don't fully understand but this might work"
- Proposing solutions before tracing data flow
- "One more fix attempt" (when already tried 2+)

**ALL of these mean: STOP. Return to Phase 1.**

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "Issue is simple" | Simple issues have root causes too |
| "Emergency, no time" | Systematic is FASTER than thrashing |
| "Just try this first" | First fix sets the pattern. Do it right. |
| "I see the problem" | Seeing symptoms ≠ understanding root cause |
| "One more fix attempt" | 3+ failures = architectural problem |

## Quick Reference

| Phase | Key Activities | Success Criteria |
|-------|---------------|------------------|
| **1. Root Cause** | Read errors, reproduce, trace across boundaries | Understand WHAT and WHY |
| **2. Pattern** | Find working examples, compare | Identify differences |
| **3. Hypothesis** | Form theory, test minimally | Confirmed or new hypothesis |
| **4. Implementation** | Create test, fix, verify | Bug resolved, all checks pass |
