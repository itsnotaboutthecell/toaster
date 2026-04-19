import React from "react";
import type { CaptionLayout } from "@/bindings";

export type CaptionMockOrientation = "horizontal" | "vertical";

interface CaptionMockFrameProps {
  orientation: CaptionMockOrientation;
  /**
   * Optional backend-computed caption layout. When supplied, the
   * mock frame renders an outline of the pill bounds at the exact
   * position that export will produce. Falls back to the schematic
   * arrow layout when absent.
   */
  layout?: CaptionLayout;
}

const STROKE = "#EEEEEE";
const STROKE_OPACITY_FRAME = 0.55;
const STROKE_OPACITY_CENTER = 0.28;
const STROKE_OPACITY_ARROW = 0.45;

// Arrow markers + center crosshair lines around a rounded-rect frame.
// Pure vector; no <text> nodes (Slice A AC-001-c forbids pixel labels).
export const CaptionMockFrame: React.FC<CaptionMockFrameProps> = ({
  orientation,
  layout,
}) => {
  const isHorizontal = orientation === "horizontal";
  const w = isHorizontal ? 1600 : 900;
  const h = isHorizontal ? 900 : 1600;

  // Frame inset so arrows have room to live outside the rect.
  const inset = Math.round(Math.min(w, h) * 0.12);
  const x0 = inset;
  const y0 = inset;
  const x1 = w - inset;
  const y1 = h - inset;
  const cx = (x0 + x1) / 2;
  const cy = (y0 + y1) / 2;
  const radius = Math.round(Math.min(x1 - x0, y1 - y0) * 0.05);

  // Dashed centerlines (dash sized vs short axis for visual parity).
  const shortAxis = Math.min(x1 - x0, y1 - y0);
  const dash = `${Math.round(shortAxis * 0.025)} ${Math.round(shortAxis * 0.02)}`;
  const arrowGap = Math.round(inset * 0.45);
  const strokeW = Math.round(shortAxis * 0.006);

  return (
    <svg
      viewBox={`0 0 ${w} ${h}`}
      preserveAspectRatio="xMidYMid meet"
      className="absolute inset-0 w-full h-full pointer-events-none"
      aria-hidden="true"
    >
      <defs>
        <marker
          id={`cap-mock-arrow-${orientation}`}
          viewBox="0 0 10 10"
          refX="5"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path
            d="M 0 0 L 10 5 L 0 10 z"
            fill={STROKE}
            fillOpacity={STROKE_OPACITY_ARROW + 0.2}
          />
        </marker>
      </defs>

      <rect
        x={x0}
        y={y0}
        width={x1 - x0}
        height={y1 - y0}
        rx={radius}
        ry={radius}
        fill="none"
        stroke={STROKE}
        strokeOpacity={STROKE_OPACITY_FRAME}
        strokeWidth={strokeW}
      />

      <line
        x1={x0}
        y1={cy}
        x2={x1}
        y2={cy}
        stroke={STROKE}
        strokeOpacity={STROKE_OPACITY_CENTER}
        strokeWidth={Math.max(1, strokeW - 1)}
        strokeDasharray={dash}
      />
      <line
        x1={cx}
        y1={y0}
        x2={cx}
        y2={y1}
        stroke={STROKE}
        strokeOpacity={STROKE_OPACITY_CENTER}
        strokeWidth={Math.max(1, strokeW - 1)}
        strokeDasharray={dash}
      />

      <line
        x1={x0}
        y1={y0 - arrowGap}
        x2={x1}
        y2={y0 - arrowGap}
        stroke={STROKE}
        strokeOpacity={STROKE_OPACITY_ARROW}
        strokeWidth={strokeW}
        markerStart={`url(#cap-mock-arrow-${orientation})`}
        markerEnd={`url(#cap-mock-arrow-${orientation})`}
      />
      <line
        x1={x0 - arrowGap}
        y1={y0}
        x2={x0 - arrowGap}
        y2={y1}
        stroke={STROKE}
        strokeOpacity={STROKE_OPACITY_ARROW}
        strokeWidth={strokeW}
        markerStart={`url(#cap-mock-arrow-${orientation})`}
        markerEnd={`url(#cap-mock-arrow-${orientation})`}
      />
      {layout && (
        <rect
          x={layout.margin_h_px * (w / layout.frame_width)}
          y={layout.margin_v_px * (h / layout.frame_height)}
          width={layout.box_width_px * (w / layout.frame_width)}
          height={(layout.font_size_px + layout.padding_y_px * 2) * (h / layout.frame_height)}
          rx={layout.radius_px * (w / layout.frame_width)}
          ry={layout.radius_px * (w / layout.frame_width)}
          fill="none"
          stroke={STROKE}
          strokeOpacity={STROKE_OPACITY_ARROW}
          strokeWidth={strokeW}
          strokeDasharray={dash}
        />
      )}
    </svg>
  );
};
