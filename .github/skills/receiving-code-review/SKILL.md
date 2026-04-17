---
name: receiving-code-review
description: 'Use when receiving code review feedback, before implementing suggestions. Requires technical evaluation and verification, not performative agreement or blind implementation.'
---

# Receiving Code Review

## Overview

Code review requires technical evaluation, not emotional performance.

**Core principle:** Verify before implementing. Ask before assuming. Technical correctness over social comfort.

## The Response Pattern

```
WHEN receiving code review feedback:

1. READ: Complete feedback without reacting
2. UNDERSTAND: Restate requirement in own words (or ask)
3. VERIFY: Check against codebase reality
4. EVALUATE: Technically sound for THIS codebase?
5. RESPOND: Technical acknowledgment or reasoned pushback
6. IMPLEMENT: One item at a time, test each
```

## Forbidden Responses

**NEVER:**
- "You're absolutely right!"
- "Great point!" / "Excellent feedback!"
- "Let me implement that now" (before verification)

**INSTEAD:**
- Restate the technical requirement
- Ask clarifying questions
- Push back with technical reasoning if wrong
- Just start working — actions speak louder than words

## Handling Unclear Feedback

```
IF any item is unclear:
  STOP — do not implement anything yet
  ASK for clarification on unclear items

WHY: Items may be related. Partial understanding = wrong implementation.
```

## Toaster Architecture Checks

Before implementing any review suggestion, verify it respects Toaster's boundaries:

| Boundary | Rule |
|----------|------|
| Business logic location | Must be in `src-tauri/src/managers/`, not frontend |
| Timeline authority | Backend keep-segment/time-mapping is single source of truth |
| Frontend role | Calls Tauri commands and renders state — no independent timeline logic |
| Video source | Never swap video element source to audio preview file |
| Timestamps | Always microseconds (matching FFmpeg AV_TIME_BASE) |
| UI text | Must use i18next keys, not hardcoded strings |

If a review suggestion violates these boundaries, push back with the specific rule.

## Evaluation Before Implementation

### From the project owner
- Implement after understanding
- Still ask if scope is unclear
- No performative agreement — skip to action

### From external reviewers or AI agents
```
BEFORE implementing:
  1. Technically correct for THIS codebase?
  2. Breaks existing functionality?
  3. Reason for current implementation?
  4. Works on all target platforms (Windows/macOS/Linux)?
  5. Does reviewer understand full context?

IF suggestion seems wrong:
  Push back with technical reasoning

IF can't easily verify:
  Say so: "I can't verify this without [X]."
```

## YAGNI Check

```
IF reviewer suggests adding features "for completeness":
  Check if the feature is actually used or needed

  IF unused: "This isn't called anywhere. Remove it (YAGNI)?"
  IF used: Then implement properly
```

## Implementation Order

For multi-item feedback:
1. Clarify anything unclear FIRST
2. Then implement in order:
   - Blocking issues (breaks, security)
   - Simple fixes (typos, imports)
   - Complex fixes (refactoring, logic)
3. Test each fix individually
4. Verify no regressions with `cargo test` and `npm run lint`

## When to Push Back

Push back when:
- Suggestion breaks existing functionality
- Reviewer lacks full context
- Violates YAGNI
- Technically incorrect for Tauri/Rust/React stack
- Violates Toaster architecture boundaries
- Conflicts with prior architectural decisions

**How to push back:**
- Use technical reasoning, not defensiveness
- Reference specific code, tests, or architectural rules
- Ask specific questions

## Acknowledging Correct Feedback

When feedback IS correct:
```
✅ "Fixed. [Brief description of what changed]"
✅ "Good catch — [specific issue]. Fixed in [location]."
✅ [Just fix it and show the result]

❌ "You're absolutely right!"
❌ "Great point!"
❌ "Thanks for catching that!"
```

Actions speak. Just fix it.

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Performative agreement | State requirement or just act |
| Blind implementation | Verify against codebase first |
| Batch without testing | One at a time, test each |
| Assuming reviewer is right | Check if it breaks things |
| Avoiding pushback | Technical correctness > comfort |
| Partial implementation | Clarify all items first |
