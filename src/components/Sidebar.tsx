import React from "react";
import { useTranslation } from "react-i18next";
import { Info, Cpu, Scissors, Wand2, SlidersHorizontal, Bug } from "lucide-react";
import toasterLogo from "../assets/toaster_text.svg";
import {
  AboutSettings,
  ModelsSettings,
  PostProcessingSettings,
  AdvancedSettings,
  DebugSettings,
} from "./settings";
import EditorView from "./editor/EditorView";
import { useSettings } from "../hooks/useSettings";
import type { AppSettings } from "@/bindings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps extends React.SVGProps<SVGSVGElement> {
  size?: number | string;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: AppSettings | null) => boolean;
}

export const SECTIONS_CONFIG = {
  editor: {
    labelKey: "sidebar.editor",
    icon: Scissors,
    component: EditorView,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  postProcessing: {
    labelKey: "sidebar.postProcessing",
    icon: Wand2,
    component: PostProcessingSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: SlidersHorizontal,
    component: AdvancedSettings,
    enabled: () => true,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: Bug,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode === true,
  },
} as const satisfies Record<string, SectionConfig>;

/**
 * Defensive fallback for deserialized/persisted `activeSection`
 * values. Returns the editor section when the key is not a valid
 * entry of `SECTIONS_CONFIG` (e.g. legacy `"export"` or
 * `"experimental"` ids saved by older builds). Kept exported so
 * consumers — App.tsx today, other entry points later — apply the
 * same rule (SSOT).
 */
export const resolveSidebarSection = (value: unknown): SidebarSection => {
  if (
    typeof value === "string" &&
    Object.prototype.hasOwnProperty.call(SECTIONS_CONFIG, value)
  ) {
    return value as SidebarSection;
  }
  return "editor";
};

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections= Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <div className="flex flex-col w-40 h-full border-e border-mid-gray/20 items-center px-2">
      <img src={toasterLogo} alt="Toaster" className="w-[144px] mx-0 my-4" />
      <div className="flex flex-col w-full items-center gap-1 pt-2 border-t border-mid-gray/20">
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <div
              key={section.id}
              className={`flex gap-2 items-center p-2 w-full rounded-lg cursor-pointer transition-colors ${
                isActive
                  ? "bg-logo-primary/80"
                  : "hover:bg-mid-gray/20 hover:opacity-100 opacity-85"
              }`}
              onClick={() => onSectionChange(section.id)}
            >
              <Icon width={24} height={24} className="shrink-0" />
              <p
                className="text-sm font-medium truncate"
                title={t(section.labelKey)}
              >
                {t(section.labelKey)}
              </p>
            </div>
          );
        })}
      </div>
    </div>
  );
};
