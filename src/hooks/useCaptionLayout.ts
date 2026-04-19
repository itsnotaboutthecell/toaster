import { useEffect, useState } from "react";
import { commands, type CaptionLayout, type Orientation } from "@/bindings";

/**
 * React hook that pipes through `commands.get_caption_layout` so the
 * preview renders with the same `CaptionLayout` that libass export
 * will use. Returns `null` while loading / on error (callers should
 * fall back to the schematic mock in that case).
 *
 * Slice B R-004 / AC-005-e.
 */
export function useCaptionLayout(
  orientation: Orientation | null,
  width: number,
  height: number,
): CaptionLayout | null {
  const [layout, setLayout] = useState<CaptionLayout | null>(null);

  useEffect(() => {
    let cancelled = false;
    if (!orientation || width <= 0 || height <= 0) {
      setLayout(null);
      return () => {
        cancelled = true;
      };
    }
    (async () => {
      const res = await commands.getCaptionLayout(orientation, { width, height });
      if (cancelled) return;
      if (res.status === "ok") setLayout(res.data);
      else setLayout(null);
    })();
    return () => {
      cancelled = true;
    };
  }, [orientation, width, height]);

  return layout;
}
