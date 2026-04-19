import { useSettings } from "./useSettings";
import type { AppSettings } from "@/bindings";

type BooleanSettingKey = {
  [K in keyof AppSettings]-?: NonNullable<AppSettings[K]> extends boolean
    ? K
    : never;
}[keyof AppSettings];

/**
 * Defence-in-depth getter for experimental booleans — frontend mouth
 * of the same SSOT rule implemented by `is_experiment_enabled` on the
 * Rust side (`src-tauri/src/settings/mod.rs`).
 *
 * When the master toggle `experimental_enabled` is `false`, this hook
 * returns `false` regardless of the stored per-flag value. When the
 * master is `true`, it returns the stored per-flag boolean verbatim.
 * Stored values are never mutated — flipping the master back on
 * restores the user's prior opt-in.
 */
export function useExperiment(key: BooleanSettingKey): boolean {
  const { getSetting } = useSettings();
  if (key === "experimental_enabled") {
    return (getSetting("experimental_enabled") as boolean) ?? false;
  }
  const master = (getSetting("experimental_enabled") as boolean) ?? false;
  if (!master) {
    return false;
  }
  return (getSetting(key) as boolean) ?? false;
}
