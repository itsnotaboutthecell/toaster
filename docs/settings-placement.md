# Settings placement heuristic

## Rule

A setting belongs in **Advanced** (Settings -> Advanced, sidebar entry) when all of these are true:

1. **Configured once per machine or project.** The user sets it during initial setup or a one-time refinement, and returns to it rarely thereafter.
2. **Not tied to a per-clip editing decision.** It does not drive moment-to-moment choices in the Editor view.
3. **Requires explanation.** It works best when accompanied by a paragraph of context (e.g. filler-word strategy, privacy guarantees, model endpoints).

A setting belongs in **Editor** (inside `EditorView.tsx`) only when it is a per-clip / per-session decision the user toggles frequently during editing (e.g. "Normalize audio for this export").

A setting belongs in **Experimental** when it is feature-flagged, potentially unstable, or being A/B tested.

A setting belongs in **Models**, **Post-Process**, or **Export** when it is the primary subject of that domain.

## Why

Putting configured-once controls (Allow Words, Discard Words) into the Editor view created visual noise on a surface the user hits dozens of times per session. The user's feedback captured the regression:

> placing the discard/allow word list in the main editor is overkill as they will only configure these settings once and never need again.

This rule exists so the next contributor does not repeat that relocation. If a setting meets the three Advanced criteria above, it goes under Settings -> Advanced. If it does not, keep it on its domain page.

## Recommendation table

See [features/advanced-menu-restoration/journal.md](../features/advanced-menu-restoration/journal.md) "Audit" section for the per-setting recommendation table produced during this feature.

## Layout invariants

The settings UI is held to four machine-enforced layout invariants, audited by
`tests/settingsUIAudit.spec.ts` and driven via `scripts/migrate/audit-settings-ui.ps1`:

1. **Outer padding.** Every settings page's outer container matches the
   canonical pattern used by `AboutSettings` (`max-w-5xl w-full mx-auto`
   plus `space-y-*`). Tolerance: 8 px on `padding-inline` and `max-width`
   against the About baseline.
2. **Two-column row.** Every `[data-testid="setting-row"]` has a
   `[data-setting-role="label"]` preceding `[data-setting-role="control"]`
   in DOM order. Mobile-portrait viewports (< 500 px wide) may collapse
   to column direction.
3. **Preview clamp.** The caption preview pane
   (`[data-testid="caption-preview-pane"]`) is ≤ 50 % of viewport width
   on desktop and ≤ 40 % of viewport height on mobile-portrait; no
   horizontal overflow at 320 px width.
4. **Settings UI contract.** Every row has a human-readable label and a
   non-empty description; numeric sliders have a sibling editable node
   (double-click-to-type affordance); no red text on dark backgrounds;
   no light-grey text on white. Mirrors AGENTS.md "Settings UI contract".

See `features/settings-ui-consistency-audit/` for the feature bundle
that landed these rules.
