import React, { Suspense, lazy } from "react";
import { useTranslation } from "react-i18next";
import { Info, Cpu, Scissors, SlidersHorizontal, Bug } from "lucide-react";
import toasterLogo from "../assets/toaster_text.svg";
import EditorView from "./editor/EditorView";
import { useSettings } from "../hooks/useSettings";
import type { AppSettings } from "@/bindings";

// Non-editor settings tabs are lazy-loaded so the initial app shell
// only pays for the editor surface. Users typically land in the editor
// and only open settings on-demand, so the extra network fetch is paid
// at the moment the user navigates there (Suspense fallback hides the
// latency). Net bundle win of ~150+ kB gzipped main chunk.
const ModelsSettings = lazy(() =>
  import("./settings").then((m) => ({ default: m.ModelsSettings })),
);
const AdvancedSettings = lazy(() =>
  import("./settings").then((m) => ({ default: m.AdvancedSettings })),
);
const AboutSettings = lazy(() =>
  import("./settings").then((m) => ({ default: m.AboutSettings })),
);
const DebugSettings = lazy(() =>
  import("./settings").then((m) => ({ default: m.DebugSettings })),
);

/** Minimal fallback for lazy settings tabs — avoids layout shift. */
const SettingsTabFallback: React.FC = () => (
  <div className="p-4 text-sm text-mid-gray/60" aria-busy="true">…</div>
);

/** Wrap a lazy settings tab in its own Suspense boundary. */
function withSuspense<P extends object>(
  Component: React.ComponentType<P>,
): React.ComponentType<P> {
  const Wrapped: React.FC<P> = (props) => (
    <Suspense fallback={<SettingsTabFallback />}>
      <Component {...props} />
    </Suspense>
  );
  Wrapped.displayName = `withSuspense(${Component.displayName ?? "Lazy"})`;
  return Wrapped;
}

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
    component: withSuspense(ModelsSettings),
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: SlidersHorizontal,
    component: withSuspense(AdvancedSettings),
    enabled: () => true,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: withSuspense(AboutSettings),
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: Bug,
    component: withSuspense(DebugSettings),
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
