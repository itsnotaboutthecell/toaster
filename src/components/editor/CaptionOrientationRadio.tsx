import React from "react";
import { useTranslation } from "react-i18next";
import type { Orientation } from "@/bindings";

export type OrientationChoice = "Desktop" | "Mobile" | "Auto";

interface CaptionOrientationRadioProps {
  value: OrientationChoice;
  onChange: (next: OrientationChoice) => void;
  /** When `value === "Auto"`, the resolved orientation derived from
   * the current video dimensions. Rendered as a muted hint next to
   * the Auto option so users can see which profile will be used. */
  resolved?: Orientation;
}

/**
 * Editor-level caption orientation selector. Slice B R-006.
 *
 * Ephemeral React state (NOT persisted). Default is `Auto`, which
 * picks Desktop for width/height >= 1.0 and Mobile otherwise. The
 * resolved value is what's passed to `get_caption_layout`.
 */
export const CaptionOrientationRadio: React.FC<CaptionOrientationRadioProps> = ({
  value,
  onChange,
  resolved,
}) => {
  const { t } = useTranslation();
  const options: OrientationChoice[] = ["Desktop", "Mobile", "Auto"];
  return (
    <fieldset
      aria-label={t("editor.captionOrientation.ariaLabel")}
      className="flex items-center gap-3 text-xs text-text/80"
    >
      <legend className="sr-only">
        {t("editor.captionOrientation.ariaLabel")}
      </legend>
      <span className="text-text/60">
        {t("editor.captionOrientation.label")}
      </span>
      {options.map((opt) => {
        const checked = value === opt;
        const showResolved = opt === "Auto" && value === "Auto" && resolved;
        return (
          <label
            key={opt}
            className={`flex items-center gap-1 cursor-pointer px-2 py-1 rounded ${
              checked ? "bg-accent/10 text-text" : "text-text/70 hover:text-text"
            }`}
          >
            <input
              type="radio"
              name="caption-orientation"
              checked={checked}
              onChange={() => onChange(opt)}
              className="accent-accent"
            />
            <span>{t(`editor.captionOrientation.option.${opt.toLowerCase()}`)}</span>
            {showResolved && (
              <span className="text-text/40">
                (
                {t(
                  `editor.captionOrientation.option.${resolved!.toLowerCase()}`,
                )}
                )
              </span>
            )}
          </label>
        );
      })}
    </fieldset>
  );
};

/**
 * Resolve `OrientationChoice` to a concrete `Orientation` using video
 * dims. Landscape (w >= h) resolves to Desktop, portrait/square to
 * Mobile. Exposed so callers (and tests) can compute the same value
 * the UI reports.
 */
export function resolveOrientation(
  choice: OrientationChoice,
  width: number,
  height: number,
): Orientation {
  if (choice === "Desktop") return "Desktop";
  if (choice === "Mobile") return "Mobile";
  if (width <= 0 || height <= 0) return "Desktop";
  return width >= height ? "Desktop" : "Mobile";
}
