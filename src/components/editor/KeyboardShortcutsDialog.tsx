import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { X, Keyboard } from "lucide-react";
import { Button } from "@/components/ui/Button";

/**
 * Visible list of every global editor shortcut. Acts as both in-app
 * documentation (rendered by `KeyboardShortcutsDialog`) and the canonical
 * reference for reviewers: if you add a new `window.addEventListener(
 * "keydown", ...)` handler elsewhere, add a row here too or it isn't
 * discoverable.
 *
 * `keys` is the raw shortcut spec — one item per platform-variant chord
 * (e.g. `["ctrl", "d"]` renders as "Ctrl + D"). The array-of-arrays form
 * renders as alternative bindings separated by "or".
 *
 * `labelKey` is an i18next key under `shortcuts.actions.*`.
 */
export type ShortcutEntry = {
  keys: string[][];
  labelKey: string;
};

export type ShortcutGroup = {
  titleKey: string;
  items: ShortcutEntry[];
};

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    titleKey: "shortcuts.groups.playback",
    items: [
      { keys: [["space"]], labelKey: "shortcuts.actions.playPause" },
      { keys: [["k"]], labelKey: "shortcuts.actions.jogPlayPause" },
      { keys: [["j"]], labelKey: "shortcuts.actions.jogBack" },
      { keys: [["l"]], labelKey: "shortcuts.actions.jogForward" },
      { keys: [["←"]], labelKey: "shortcuts.actions.seekBack" },
      { keys: [["→"]], labelKey: "shortcuts.actions.seekForward" },
    ],
  },
  {
    titleKey: "shortcuts.groups.editing",
    items: [
      {
        keys: [["delete"], ["backspace"], ["ctrl", "d"]],
        labelKey: "shortcuts.actions.deleteWord",
      },
      { keys: [["ctrl", "m"]], labelKey: "shortcuts.actions.silenceWord" },
      {
        keys: [["ctrl", "shift", "s"]],
        labelKey: "shortcuts.actions.splitWord",
      },
    ],
  },
  {
    titleKey: "shortcuts.groups.selection",
    items: [
      { keys: [["ctrl", "a"]], labelKey: "shortcuts.actions.selectAll" },
      { keys: [["esc"]], labelKey: "shortcuts.actions.deselect" },
    ],
  },
  {
    titleKey: "shortcuts.groups.history",
    items: [
      { keys: [["ctrl", "z"]], labelKey: "shortcuts.actions.undo" },
      { keys: [["ctrl", "shift", "z"]], labelKey: "shortcuts.actions.redo" },
    ],
  },
  {
    titleKey: "shortcuts.groups.general",
    items: [
      { keys: [["?"], ["f1"]], labelKey: "shortcuts.actions.openHelp" },
      {
        keys: [["ctrl", "shift", "d"]],
        labelKey: "shortcuts.actions.toggleDebug",
      },
    ],
  },
];

/**
 * Render a single `["ctrl","shift","d"]` chord as keycap spans joined by "+".
 * On macOS, `ctrl` is displayed as the macOS convention (still "Ctrl" here —
 * full cross-platform mapping lives in `lib/utils/keyboard.ts`; we don't
 * duplicate it for display because these chords are OS-agnostic literals
 * matched against `KeyboardEvent.key` + `ctrlKey`/`metaKey` in EditorView).
 */
const Chord: React.FC<{ chord: string[] }> = ({ chord }) => (
  <span className="inline-flex items-center gap-1">
    {chord.map((k, i) => (
      <React.Fragment key={i}>
        {i > 0 && <span className="text-mid-gray text-xs">+</span>}
        <kbd className="px-2 py-0.5 rounded border border-mid-gray/30 bg-mid-gray/10 text-xs font-mono min-w-[1.5rem] text-center">
          {k === "ctrl"
            ? "Ctrl"
            : k === "shift"
              ? "Shift"
              : k === "alt"
                ? "Alt"
                : k === "esc"
                  ? "Esc"
                  : k === "space"
                    ? "Space"
                    : k === "delete"
                      ? "Del"
                      : k === "backspace"
                        ? "⌫"
                        : k.length === 1
                          ? k.toUpperCase()
                          : k.toUpperCase()}
        </kbd>
      </React.Fragment>
    ))}
  </span>
);

interface KeyboardShortcutsDialogProps {
  open: boolean;
  onClose: () => void;
}

const KeyboardShortcutsDialog: React.FC<KeyboardShortcutsDialogProps> = ({
  open,
  onClose,
}) => {
  const { t } = useTranslation();

  useEffect(() => {
    if (!open) return;
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    document.addEventListener("keydown", handleEsc, true);
    return () => document.removeEventListener("keydown", handleEsc, true);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="shortcuts-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onClick={onClose}
    >
      <div
        className="bg-background border border-mid-gray/20 rounded-xl shadow-2xl max-w-2xl w-full max-h-[85vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-6 py-4 border-b border-mid-gray/20">
          <div className="flex items-center gap-2">
            <Keyboard size={18} className="text-logo-primary" />
            <h2 id="shortcuts-title" className="text-lg font-medium">
              {t("shortcuts.title")}
            </h2>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={onClose}
            aria-label={t("shortcuts.close")}
            className="p-1"
          >
            <X size={18} />
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-6">
          {SHORTCUT_GROUPS.map((group) => (
            <section key={group.titleKey}>
              <h3 className="text-xs uppercase tracking-wider text-mid-gray mb-2 font-medium">
                {t(group.titleKey)}
              </h3>
              <ul className="space-y-1">
                {group.items.map((item, idx) => (
                  <li
                    key={idx}
                    className="flex items-center justify-between py-1.5 text-sm"
                  >
                    <span>{t(item.labelKey)}</span>
                    <span className="flex items-center gap-2">
                      {item.keys.map((chord, i) => (
                        <React.Fragment key={i}>
                          {i > 0 && (
                            <span className="text-mid-gray text-xs">
                              {t("shortcuts.or")}
                            </span>
                          )}
                          <Chord chord={chord} />
                        </React.Fragment>
                      ))}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>

        <div className="px-6 py-3 border-t border-mid-gray/20 text-xs text-mid-gray">
          {t("shortcuts.footer")}
        </div>
      </div>
    </div>
  );
};

export default KeyboardShortcutsDialog;
