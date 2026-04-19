import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import type { CaptionProfile, CaptionProfileSet } from "@/bindings";
import { CaptionPreviewPane } from "./CaptionProfileShared";
import { CaptionProfileForm } from "./CaptionProfileForm";
import type { CaptionMockOrientation } from "./CaptionMockFrame";

interface CaptionSettingsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

type OrientationTab = "desktop" | "mobile";

const DEFAULT_DESKTOP: CaptionProfile = {
  font_size: 40,
  bg_color: "#000000B3",
  text_color: "#FFFFFF",
  position: 90,
  font_family: "Inter",
  radius_px: 0,
  padding_x_px: 12,
  padding_y_px: 4,
  max_width_percent: 90,
};

const DEFAULT_MOBILE: CaptionProfile = {
  font_size: 48,
  bg_color: "#000000B3",
  text_color: "#FFFFFF",
  position: 80,
  font_family: "Inter",
  radius_px: 8,
  padding_x_px: 14,
  padding_y_px: 6,
  max_width_percent: 80,
};

/**
 * Caption settings surface split per orientation (Slice B of
 * `caption-profiles-persistence`). Previously flat `caption_*` fields
 * now live on `AppSettings.caption_profiles.{desktop, mobile}`. The
 * shell renders a Desktop|Mobile tab selector; each tab wraps
 * `CaptionProfileForm` bound to the matching profile.
 */
export const CaptionSettings: React.FC<CaptionSettingsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const profileSet =
      (getSetting("caption_profiles") as CaptionProfileSet | undefined) ?? {
        desktop: DEFAULT_DESKTOP,
        mobile: DEFAULT_MOBILE,
      };

    const [tab, setTab] = useState<OrientationTab>("desktop");
    const [previewOrientation, setPreviewOrientation] =
      useState<CaptionMockOrientation>("horizontal");

    const activeProfile = tab === "desktop" ? profileSet.desktop : profileSet.mobile;

    const handleChange = (patch: Partial<CaptionProfile>) => {
      const merged: CaptionProfile = { ...activeProfile, ...patch };
      const next: CaptionProfileSet = {
        desktop: tab === "desktop" ? merged : profileSet.desktop,
        mobile: tab === "mobile" ? merged : profileSet.mobile,
      };
      updateSetting("caption_profiles", next);
    };

    const handleTabChange = (next: OrientationTab) => {
      setTab(next);
      setPreviewOrientation(next === "desktop" ? "horizontal" : "vertical");
    };

    const disabled = isUpdating("caption_profiles");

    return (
      <>
        <div
          role="tablist"
          aria-label={t("settings.captions.tabs.ariaLabel")}
          className="mb-3 flex items-center gap-1 border-b border-mid-gray/30"
        >
          {(["desktop", "mobile"] as const).map((key) => (
            <button
              key={key}
              role="tab"
              aria-selected={tab === key}
              onClick={() => handleTabChange(key)}
              className={`px-4 py-2 text-xs font-medium border-b-2 transition-colors ${
                tab === key
                  ? "border-accent text-text"
                  : "border-transparent text-text/60 hover:text-text"
              }`}
            >
              {t(`settings.captions.tabs.${key}`)}
            </button>
          ))}
        </div>

        <p className="mb-3 text-xs text-text/60">
          {t(`settings.captions.tabs.${tab}Description`)}
        </p>

        <CaptionPreviewPane
          profile={activeProfile}
          orientation={previewOrientation}
          onOrientationChange={setPreviewOrientation}
        />

        <CaptionProfileForm
          profile={activeProfile}
          onChange={handleChange}
          descriptionMode={descriptionMode}
          grouped={grouped}
          disabled={disabled}
        />
      </>
    );
  },
);
