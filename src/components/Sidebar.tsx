import React from "react";
import { useTranslation } from "react-i18next";
import { Cog, History, Info, Cpu, Scissors } from "lucide-react";
import toasterLogo from "../../toaster_text.svg";
import {
  AdvancedSettings,
  HistorySettings,
  AboutSettings,
  ModelsSettings,
} from "./settings";
import EditorView from "./editor/EditorView";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps extends React.SVGProps<SVGSVGElement> {
  size?: number | string;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: () => boolean;
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
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();

  const availableSections= Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled())
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <div className="flex flex-col w-40 h-full border-e border-mid-gray/20 items-center px-2">
      <img src={toasterLogo} alt="Toaster" className="w-[120px] m-4" />
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
