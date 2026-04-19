/**
 * Single source of truth for experimental features surfaced in
 * Settings -> Experimental. To add a new experiment, append one
 * entry here and add the corresponding i18n keys under
 * `experiments.<id>.label` / `.description` in every locale.
 *
 * No other source file change is required (panel + sidebar entry
 * iterate this registry).
 */
import type { AppSettings } from "@/bindings";

type BooleanSettingKey = {
  [K in keyof AppSettings]-?: NonNullable<AppSettings[K]> extends boolean
    ? K
    : never;
}[keyof AppSettings];

export interface Experiment {
  id: string;
  settingsKey: BooleanSettingKey;
  labelKey: string;
  descriptionKey: string;
  feedbackUrl: string;
}

export const EXPERIMENTS_FEEDBACK_URL =
  "https://github.com/itsnotaboutthecell/toaster/issues/new?labels=experimental-feedback&template=experimental_feedback.md";

export const experiments: readonly Experiment[] = [
  {
    id: "simplifyMode",
    settingsKey: "experimental_simplify_mode",
    labelKey: "experiments.simplifyMode.label",
    descriptionKey: "experiments.simplifyMode.description",
    feedbackUrl: EXPERIMENTS_FEEDBACK_URL,
  },
] as const;
