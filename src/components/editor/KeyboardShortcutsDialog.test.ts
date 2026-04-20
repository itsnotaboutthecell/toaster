import { describe, it, expect } from "vitest";
import { SHORTCUT_GROUPS } from "./KeyboardShortcutsDialog";

/**
 * The shortcut-group registry is the canonical reference for every global
 * keyboard shortcut wired in EditorView. If a row here drifts from the
 * actual handler it silently misleads users. These tests catch the two
 * most common regressions:
 *
 *   1. i18n key namespace drift (shortcuts.* vs. hardcoded English)
 *   2. Duplicate bindings (two rows claiming the same chord)
 *
 * If you add a handler to EditorView, add the row here and add a case to
 * the "documented chords" set in this test.
 */
describe("KeyboardShortcutsDialog SHORTCUT_GROUPS", () => {
  it("uses only i18n keys under the shortcuts.* namespace", () => {
    for (const group of SHORTCUT_GROUPS) {
      expect(group.titleKey.startsWith("shortcuts.groups.")).toBe(true);
      for (const item of group.items) {
        expect(item.labelKey.startsWith("shortcuts.actions.")).toBe(true);
      }
    }
  });

  it("has no duplicate chord bindings across all groups", () => {
    const seen = new Map<string, string>();
    for (const group of SHORTCUT_GROUPS) {
      for (const item of group.items) {
        for (const chord of item.keys) {
          const key = chord.join("+");
          if (seen.has(key)) {
            throw new Error(
              `Duplicate chord "${key}" bound to both "${seen.get(key)}" and "${item.labelKey}"`,
            );
          }
          seen.set(key, item.labelKey);
        }
      }
    }
  });

  it("documents the canonical EditorView chords", () => {
    // Flatten to a Set of chord strings for O(1) lookup.
    const chords = new Set<string>();
    for (const group of SHORTCUT_GROUPS) {
      for (const item of group.items) {
        for (const chord of item.keys) chords.add(chord.join("+"));
      }
    }

    // Handlers wired in src/components/editor/EditorView.tsx:
    const expected = [
      "space",
      "k",
      "j",
      "l",
      "←",
      "→",
      "delete",
      "backspace",
      "ctrl+d",
      "ctrl+m",
      "ctrl+shift+s",
      "ctrl+a",
      "esc",
      "ctrl+z",
      "ctrl+shift+z",
      "?",
      "f1",
      "ctrl+shift+d",
    ];

    for (const chord of expected) {
      expect(chords.has(chord)).toBe(true);
    }
  });

  it("groups shortcuts into at least the five expected sections", () => {
    const titles = SHORTCUT_GROUPS.map((g) => g.titleKey);
    expect(titles).toEqual(
      expect.arrayContaining([
        "shortcuts.groups.playback",
        "shortcuts.groups.editing",
        "shortcuts.groups.selection",
        "shortcuts.groups.history",
        "shortcuts.groups.general",
      ]),
    );
  });
});
