import React, { useEffect, useMemo, useState } from "react";
import type { CaptionBlock, Rgba, Word } from "@/bindings";
import { commands } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";

interface CaptionOverlayProps {
  currentTime: number;
  words: Word[];
  enabled: boolean;
  videoRef?: React.RefObject<HTMLVideoElement | null>;
}

/**
 * Find the block active at the given time via binary search.
 */
function findBlockAtTime(
  blocks: CaptionBlock[],
  timeUs: number,
): CaptionBlock | null {
  let lo = 0;
  let hi = blocks.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >>> 1;
    const b = blocks[mid];
    if (timeUs < b.start_us) {
      hi = mid - 1;
    } else if (timeUs > b.end_us) {
      lo = mid + 1;
    } else {
      return b;
    }
  }
  return null;
}

export function rgbaToCss(c: Rgba): string {
  return `rgba(${c.r},${c.g},${c.b},${(c.a / 255).toFixed(3)})`;
}

/**
 * Pure caption-pill renderer. The single source of truth for how a
 * caption box appears in any preview surface (player overlay AND the
 * settings-panel preview pane). All sizes are CSS pixels - callers
 * pre-scale by whatever video-pixel-to-CSS-pixel ratio applies. See
 * `managers/captions/layout.rs` for the export-side authority.
 */
export interface CaptionPillProps {
  lines: string[];
  fontCss: string;
  fontSizePx: number;
  lineHeightPx: number;
  textColor: Rgba;
  background: Rgba;
  /** Symmetric padding inside the box, mirroring libass `Outline`. */
  paddingPx: number;
  /** Distance from the bottom of the positioned ancestor, CSS px. */
  bottomPx: number;
  /** Horizontal offset for centering inside a fitted/letterboxed area. */
  marginLeftPx: number;
  /** Optional rounded-corner radius. Player passes `undefined` so its
   *  byte-identical output is preserved; the settings preview passes
   *  the user's caption_radius_px so AC-005-c (radius slider) works. */
  borderRadiusPx?: number;
}

export const CaptionPill: React.FC<CaptionPillProps> = ({
  lines,
  fontCss,
  fontSizePx,
  lineHeightPx,
  textColor,
  background,
  paddingPx,
  bottomPx,
  marginLeftPx,
  borderRadiusPx,
}) => {
  const boxStyle: React.CSSProperties = {
    position: "absolute",
    left: "50%",
    bottom: `${bottomPx}px`,
    transform: "translateX(-50%)",
    marginLeft: `${marginLeftPx}px`,
    background: rgbaToCss(background),
    color: rgbaToCss(textColor),
    fontFamily: fontCss,
    fontSize: `${fontSizePx}px`,
    lineHeight: `${lineHeightPx}px`,
    padding: `${paddingPx}px`,
    pointerEvents: "none",
    textAlign: "center",
    whiteSpace: "pre",
    ...(borderRadiusPx !== undefined && borderRadiusPx > 0
      ? { borderRadius: `${borderRadiusPx}px` }
      : {}),
  };

  return (
    <div style={boxStyle}>
      {lines.map((line, i) => (
        <div key={i}>{line}</div>
      ))}
    </div>
  );
};

/**
 * Compute the `object-fit: contain` visible video rectangle inside the
 * element box. When the element's aspect ratio differs from the video's
 * intrinsic aspect (letterbox or pillarbox), `getBoundingClientRect()`
 * describes the element, not the actual drawn video. Scaling caption
 * geometry by the element box over-reports size; scale by the fitted
 * rect instead.
 */
function fittedVideoRect(
  elementW: number,
  elementH: number,
  videoW: number,
  videoH: number,
): { w: number; h: number; offsetX: number; offsetY: number } {
  if (videoW <= 0 || videoH <= 0 || elementW <= 0 || elementH <= 0) {
    return { w: elementW, h: elementH, offsetX: 0, offsetY: 0 };
  }
  const elementAspect = elementW / elementH;
  const videoAspect = videoW / videoH;
  if (elementAspect > videoAspect) {
    // Element is wider than video → pillarbox (bars on left/right).
    const h = elementH;
    const w = h * videoAspect;
    return { w, h, offsetX: (elementW - w) / 2, offsetY: 0 };
  }
  // Element is narrower (or equal) → letterbox (bars on top/bottom).
  const w = elementW;
  const h = w / videoAspect;
  return { w, h, offsetX: 0, offsetY: (elementH - h) / 2 };
}

/**
 * Caption overlay that consumes the authoritative `CaptionBlock`
 * stream from the backend. Geometry is in video pixels; we scale to
 * the **fitted** video rectangle inside the `<video>` element (i.e.
 * after `object-fit: contain` letterboxing) so the preview visually
 * matches the burned-in export pixel-for-pixel — same font, same wrap,
 * same padding. Rounded corners were dropped in tandem with the export
 * switching to libass's native `BorderStyle=3` opaque box. See
 * `managers/captions/layout.rs` for the single source of truth.
 */
const CaptionOverlay: React.FC<CaptionOverlayProps> = ({
  currentTime,
  words,
  enabled,
  videoRef,
}) => {
  const [blocks, setBlocks] = useState<CaptionBlock[]>([]);
  const { getSetting } = useSettings();
  // Refetch layout when the user edits the caption profiles in
  // Advanced. Backend layout is the SSOT (see
  // managers/captions/layout.rs); we just re-ask it for its current
  // answer whenever the serialized profile set changes.
  const profilesFingerprint = JSON.stringify(
    getSetting("caption_profiles") ?? null,
  );
  const [fit, setFit] = useState<{
    w: number;
    h: number;
    offsetX: number;
    offsetY: number;
  }>({ w: 0, h: 0, offsetX: 0, offsetY: 0 });

  const wordsFingerprint = useMemo(
    () => words.map((w) => `${w.text}|${w.deleted}|${w.silenced}`).join(","),
    [words],
  );

  // Refetch blocks when the word list changes.
  useEffect(() => {
    let cancelled = false;
    if (!enabled || words.length === 0) {
      setBlocks([]);
      return;
    }
    commands
      .getCaptionBlocks("Source")
      .then((next) => {
        if (!cancelled) setBlocks(next);
      })
      .catch(() => {
        // Command may fail during startup if media isn't loaded yet.
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, wordsFingerprint, words.length, profilesFingerprint]);

  // Track the visible (contain-fitted) video rect. ResizeObserver fires
  // for element box changes; `loadedmetadata` fires when intrinsic size
  // becomes known.
  useEffect(() => {
    const video = videoRef?.current;
    if (!video) return;
    const update = () => {
      const rect = video.getBoundingClientRect();
      const vw = video.videoWidth;
      const vh = video.videoHeight;
      setFit(fittedVideoRect(rect.width, rect.height, vw, vh));
    };
    update();
    const obs = new ResizeObserver(update);
    obs.observe(video);
    video.addEventListener("loadedmetadata", update);
    return () => {
      obs.disconnect();
      video.removeEventListener("loadedmetadata", update);
    };
  }, [videoRef]);

  const block = useMemo(() => {
    if (!enabled || blocks.length === 0) return null;
    return findBlockAtTime(blocks, currentTime * 1_000_000);
  }, [enabled, currentTime, blocks]);

  if (!block) return null;

  // Scale: video-pixel → CSS-pixel against the fitted video rectangle
  // (post object-fit: contain), not the element box. Falls back to 1:1
  // when we don't know the rendered size yet.
  const scale = fit.h > 0 ? fit.h / block.frame_height : 1;

  // Export uses `max(padding_x, padding_y)` as symmetric libass `Outline`
  // (see `managers/captions/ass.rs::box_padding_px`). Preview mirrors
  // that contract so both paths render the same geometric relationship
  // between text and its background box.
  const paddingPx = Math.max(block.padding_x_px, block.padding_y_px) * scale;

  return (
    <CaptionPill
      lines={block.lines}
      fontCss={block.font_css}
      fontSizePx={block.font_size_px * scale}
      lineHeightPx={block.line_height_px * scale}
      textColor={block.text_color}
      background={block.background}
      paddingPx={paddingPx}
      bottomPx={fit.offsetY + block.margin_v_px * scale}
      marginLeftPx={fit.offsetX}
    />
  );
};

export default CaptionOverlay;
