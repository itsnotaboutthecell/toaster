import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { CaptionPill } from "../../player/CaptionOverlay";
import { Select } from "../../ui/Select";
import type { CaptionFontFamily, CaptionProfile, Rgba } from "@/bindings";
import { CaptionMockFrame, type CaptionMockOrientation } from "./CaptionMockFrame";

// CSS font stacks must mirror src-tauri/src/managers/captions/fonts.rs
// (the canonical font table). Adding a font here without adding it
// there would re-introduce dual-path drift.
export const FONT_CSS: Record<CaptionFontFamily, string> = {
  Inter: "Inter, system-ui, sans-serif",
  Roboto: "Roboto, system-ui, sans-serif",
  SystemUi: "system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
};

export function hexToRgba(hex: string): Rgba {
  const m = hex.match(/^#([0-9a-fA-F]{6})([0-9a-fA-F]{2})?$/);
  if (!m) return { r: 255, g: 255, b: 255, a: 255 };
  const r = parseInt(m[1].slice(0, 2), 16);
  const g = parseInt(m[1].slice(2, 4), 16);
  const b = parseInt(m[1].slice(4, 6), 16);
  const a = m[2] ? parseInt(m[2], 16) : 255;
  return { r, g, b, a };
}

interface SliderWithInputProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix: string;
  onChange: (value: number) => void;
  disabled?: boolean;
}

export const SliderWithInput: React.FC<SliderWithInputProps> = ({
  value,
  min,
  max,
  step = 1,
  suffix,
  onChange,
  disabled,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState(String(value));
  const [isDragging, setIsDragging] = useState(false);
  const [localValue, setLocalValue] = useState(value);

  useEffect(() => {
    if (!isDragging) setLocalValue(value);
  }, [value, isDragging]);

  const handleCommit = () => {
    const parsed = parseInt(editValue);
    if (!isNaN(parsed)) onChange(Math.min(max, Math.max(min, parsed)));
    setIsEditing(false);
  };

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const next = parseInt(e.target.value);
    setLocalValue(next);
    onChange(next);
  };

  const handleDragStart = () => setIsDragging(true);
  const handleDragEnd = () => {
    setIsDragging(false);
    // Live-drag already calls onChange on every change; commit once more
    // only if a non-drag path (e.g. keyboard arrow on focused slider)
    // changed localValue without hitting handleSliderChange's onChange.
    if (localValue !== value) onChange(localValue);
  };

  const displayValue = isDragging ? localValue : value;
  const pct = ((displayValue - min) / (max - min)) * 100;

  return (
    <div className="flex items-center gap-3">
      <div className="relative w-40 h-6 flex items-center">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={displayValue}
          onChange={handleSliderChange}
          onMouseDown={handleDragStart}
          onMouseUp={handleDragEnd}
          onMouseLeave={(e) => {
            if (isDragging && e.buttons === 0) handleDragEnd();
          }}
          onTouchStart={handleDragStart}
          onTouchEnd={handleDragEnd}
          disabled={disabled}
          className="w-full cursor-pointer outline-none appearance-none"
          style={{
            background: `linear-gradient(to right, #E8A838 0%, #E8A838 ${pct}%, #d1d5db ${pct}%, #d1d5db 100%)`,
            cursor: disabled ? "not-allowed" : "pointer",
            width: "100%",
            height: "6px",
            WebkitAppearance: "none",
            borderRadius: "3px",
          }}
        />
      </div>
      {isEditing ? (
        <input
          type="number"
          inputMode="numeric"
          min={min}
          max={max}
          step={step}
          value={editValue}
          onChange={(e) => {
            const v = e.target.value;
            if (/^\d*$/.test(v)) setEditValue(v);
          }}
          onBlur={handleCommit}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleCommit();
            if (e.key === "Escape") setIsEditing(false);
          }}
          autoFocus
          className="w-14 px-2 py-0.5 text-xs rounded border border-mid-gray/30 bg-background text-text font-mono text-right"
        />
      ) : (
        <input
          type="number"
          min={min}
          max={max}
          step={step}
          value={displayValue}
          onChange={(e) => {
            const n = parseInt(e.target.value, 10);
            if (!Number.isNaN(n)) onChange(Math.max(min, Math.min(max, n)));
          }}
          onDoubleClick={() => {
            setEditValue(String(displayValue));
            setIsEditing(true);
          }}
          disabled={disabled}
          className="w-14 px-2 py-0.5 text-xs rounded border border-mid-gray/30 bg-background text-text font-mono text-right cursor-text"
          title="Type a value or double-click to focus-edit"
          aria-label="Numeric value"
        />
      )}
      {suffix && (
        <span className="text-xs text-mid-gray select-none">{suffix}</span>
      )}
    </div>
  );
};

type SampleKey = "short" | "twoLine" | "long";

// Virtual frame the caption settings are calibrated against. Preview-only
// state; MUST NOT be plumbed through any Tauri command (Slice A SSOT rule).
const VIRTUAL_FRAME_SHORT = 1080;
const VIRTUAL_FRAME_LONG = 1920;
const HORIZONTAL_ASPECT = VIRTUAL_FRAME_LONG / VIRTUAL_FRAME_SHORT;
const VERTICAL_ASPECT = VIRTUAL_FRAME_SHORT / VIRTUAL_FRAME_LONG;

interface CaptionPreviewPaneProps {
  profile: CaptionProfile;
  orientation: CaptionMockOrientation;
  onOrientationChange: (next: CaptionMockOrientation) => void;
}

export const CaptionPreviewPane: React.FC<CaptionPreviewPaneProps> = ({
  profile,
  orientation,
  onOrientationChange,
}) => {
  const { t } = useTranslation();
  const [selectedSampleKey, setSelectedSampleKey] = useState<SampleKey>("short");
  const containerRef = useRef<HTMLDivElement>(null);
  const [containerSize, setContainerSize] = useState({ w: 0, h: 0 });

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const update = () => {
      const r = el.getBoundingClientRect();
      setContainerSize({ w: r.width, h: r.height });
    };
    update();
    const obs = new ResizeObserver(update);
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  const samples: Record<SampleKey, string> = {
    short: t("settings.captions.preview.sample.short"),
    twoLine: t("settings.captions.preview.sample.twoLine"),
    long: t("settings.captions.preview.sample.long"),
  };

  const containerShort = Math.min(containerSize.w, containerSize.h);
  const scale = containerShort > 0 ? containerShort / VIRTUAL_FRAME_SHORT : 0;
  const lines = samples[selectedSampleKey].split("\n");
  const paddingPx = Math.max(profile.padding_x_px, profile.padding_y_px) * scale;
  const lineHeightPx = profile.font_size * 1.2 * scale;
  const bottomPx = ((100 - profile.position) / 100) * containerSize.h;
  const showPill = scale > 0;

  const isVertical = orientation === "vertical";
  const screenMaxWidth = isVertical ? "320px" : undefined;

  return (
    <div
      className="mb-4 w-full rounded-xl border border-mid-gray/20 bg-background p-4"
      data-testid="caption-preview-pane"
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h3 className="text-xs font-medium uppercase tracking-wide text-text/70">
          {t("settings.captions.preview.heading")}
        </h3>
        <div className="flex flex-wrap items-center gap-3">
          <div className="flex items-center gap-2">
            <label className="text-xs text-text/60">
              {t("settings.captions.preview.orientation.label")}
            </label>
            <div className="min-w-[120px]">
              <Select
                value={orientation}
                options={[
                  { value: "horizontal", label: t("settings.captions.preview.orientation.horizontal") },
                  { value: "vertical", label: t("settings.captions.preview.orientation.vertical") },
                ]}
                onChange={(v) => {
                  if (v === "horizontal" || v === "vertical") onOrientationChange(v);
                }}
              />
            </div>
          </div>
          <div className="flex items-center gap-2">
            <label className="text-xs text-text/60">
              {t("settings.captions.preview.sampleLegend")}
            </label>
            <div className="min-w-[140px]">
              <Select
                value={selectedSampleKey}
                options={[
                  { value: "short", label: t("settings.captions.preview.sample.short") },
                  { value: "twoLine", label: t("settings.captions.preview.sample.twoLine").replace(/\n/g, " / ") },
                  { value: "long", label: t("settings.captions.preview.sample.long").replace(/\n/g, " / ") },
                ]}
                onChange={(v) => {
                  if (v === "short" || v === "twoLine" || v === "long") setSelectedSampleKey(v);
                }}
              />
            </div>
          </div>
        </div>
      </div>
      <div
        className="mx-auto w-full rounded-[20px] bg-[#000000]/85 p-2 shadow-inner"
        style={{ maxWidth: screenMaxWidth ?? "36rem" }}
      >
        <div
          ref={containerRef}
          className="relative w-full overflow-hidden rounded-[12px] border border-mid-gray/20"
          style={{
            aspectRatio: `${isVertical ? VERTICAL_ASPECT : HORIZONTAL_ASPECT}`,
            backgroundColor: "#1a1a1a",
          }}
        >
          <CaptionMockFrame orientation={orientation} />
          {showPill && (
            <CaptionPill
              lines={lines}
              fontCss={FONT_CSS[profile.font_family]}
              fontSizePx={profile.font_size * scale}
              lineHeightPx={lineHeightPx}
              textColor={hexToRgba(profile.text_color)}
              background={hexToRgba(profile.bg_color)}
              paddingPx={paddingPx}
              bottomPx={bottomPx}
              marginLeftPx={0}
              borderRadiusPx={profile.radius_px * scale}
            />
          )}
        </div>
      </div>
    </div>
  );
};
