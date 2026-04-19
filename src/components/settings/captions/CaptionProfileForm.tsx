import React from "react";
import { useTranslation } from "react-i18next";
import { ColorPicker } from "../../ui/ColorPicker";
import { Select } from "../../ui/Select";
import { SettingContainer } from "../../ui/SettingContainer";
import type { CaptionFontFamily, CaptionProfile } from "@/bindings";
import { SliderWithInput } from "./CaptionProfileShared";

interface CaptionProfileFormProps {
  profile: CaptionProfile;
  onChange: (patch: Partial<CaptionProfile>) => void;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  disabled?: boolean;
}

/**
 * Renders the 9 profile-level fields for a single caption profile
 * (desktop or mobile). Writes back through `onChange` which the tab
 * wrapper plumbs into `commands.setCaptionProfile(orientation, ..., App)`.
 *
 * The form is orientation-agnostic; the tab wrappers pick which
 * profile to pass in.
 */
export const CaptionProfileForm: React.FC<CaptionProfileFormProps> = ({
  profile,
  onChange,
  descriptionMode = "tooltip",
  grouped = false,
  disabled,
}) => {
  const { t } = useTranslation();

  const bgColorHex = profile.bg_color;
  const bgColorBase = bgColorHex.slice(0, 7);
  const bgAlphaHex = bgColorHex.length > 7 ? bgColorHex.slice(7, 9) : "FF";
  const bgTransparency = Math.round((parseInt(bgAlphaHex, 16) / 255) * 100);

  const handleTransparencyChange = (pct: number) => {
    const alpha = Math.round((pct / 100) * 255)
      .toString(16)
      .padStart(2, "0")
      .toUpperCase();
    onChange({ bg_color: bgColorBase + alpha });
  };

  return (
    <>
      <SettingContainer
        title={t("settings.controls.captionSettings.position")}
        description={t("settings.controls.captionSettings.positionDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={profile.position}
          min={0}
          max={100}
          suffix="%"
          onChange={(v) => onChange({ position: v })}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.fontSize")}
        description={t("settings.controls.captionSettings.fontSizeDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={profile.font_size}
          min={12}
          max={72}
          suffix="px"
          onChange={(v) => onChange({ font_size: v })}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.bgTransparency")}
        description={t("settings.controls.captionSettings.bgTransparencyDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={bgTransparency}
          min={0}
          max={100}
          suffix="%"
          onChange={handleTransparencyChange}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.bgColor")}
        description={t("settings.controls.captionSettings.bgColorDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <ColorPicker
          value={bgColorBase}
          onChange={(color) => {
            const alpha = bgColorHex.length > 7 ? bgColorHex.slice(7, 9) : "B3";
            onChange({ bg_color: color + alpha });
          }}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.textColor")}
        description={t("settings.controls.captionSettings.textColorDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <ColorPicker
          value={profile.text_color}
          onChange={(color) => onChange({ text_color: color })}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.fontFamily")}
        description={t("settings.controls.captionSettings.fontFamilyDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Select
          value={profile.font_family}
          options={[
            { value: "Inter", label: t("settings.controls.captionSettings.fontInter") },
            { value: "Roboto", label: t("settings.controls.captionSettings.fontRoboto") },
            { value: "SystemUi", label: t("settings.controls.captionSettings.fontSystemUi") },
          ]}
          onChange={(v) => {
            if (v) onChange({ font_family: v as CaptionFontFamily });
          }}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.cornerRadius")}
        description={t("settings.controls.captionSettings.cornerRadiusDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={profile.radius_px}
          min={0}
          max={48}
          suffix="px"
          onChange={(v) => onChange({ radius_px: v })}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.paddingHorizontal")}
        description={t("settings.controls.captionSettings.paddingHorizontalDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={profile.padding_x_px}
          min={0}
          max={64}
          suffix="px"
          onChange={(v) => onChange({ padding_x_px: v })}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.paddingVertical")}
        description={t("settings.controls.captionSettings.paddingVerticalDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={profile.padding_y_px}
          min={0}
          max={32}
          suffix="px"
          onChange={(v) => onChange({ padding_y_px: v })}
          disabled={disabled}
        />
      </SettingContainer>

      <SettingContainer
        title={t("settings.controls.captionSettings.maxWidth")}
        description={t("settings.controls.captionSettings.maxWidthDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <SliderWithInput
          value={profile.max_width_percent}
          min={20}
          max={100}
          suffix="%"
          onChange={(v) => onChange({ max_width_percent: v })}
          disabled={disabled}
        />
      </SettingContainer>
    </>
  );
};
