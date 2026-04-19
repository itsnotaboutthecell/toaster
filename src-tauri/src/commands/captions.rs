//! Tauri commands for caption profiles + layout.
//!
//! Slice B of the caption work (`caption-profiles-persistence`). The
//! preview frontend and the libass export path BOTH read layout through
//! `managers::captions::compute_caption_layout`, so
//! `get_caption_layout` here and the export composer's call in
//! `commands::export` produce byte-identical [`CaptionLayout`] values
//! for the same inputs. The
//! `preview_and_export_layouts_are_byte_identical` test is the CI gate
//! that enforces this.

use crate::commands::project::CurrentProjectStore;
use crate::managers::captions::{compute_caption_layout, CaptionLayout};
use crate::settings::{
    self, CaptionProfile, CaptionProfileSet, Orientation, ProfileScope, VideoDims,
};
use tauri::{AppHandle, State};

fn project_profiles(project: &State<CurrentProjectStore>) -> Option<CaptionProfileSet> {
    let guard = project.0.lock().ok()?;
    guard.clone()
}

fn select(profile_set: &CaptionProfileSet, orientation: Orientation) -> CaptionProfile {
    match orientation {
        Orientation::Desktop => profile_set.desktop.clone(),
        Orientation::Mobile => profile_set.mobile.clone(),
    }
}

fn write(
    profile_set: &mut CaptionProfileSet,
    orientation: Orientation,
    profile: CaptionProfile,
) {
    match orientation {
        Orientation::Desktop => profile_set.desktop = profile,
        Orientation::Mobile => profile_set.mobile = profile,
    }
}

/// Read the effective caption profile for the given orientation.
///
/// Prefers project-level overrides; falls back to app-level settings.
/// Slice B R-005 / AC-005-a / AC-005-b.
#[tauri::command]
#[specta::specta]
pub fn get_caption_profile(
    app: AppHandle,
    project: State<CurrentProjectStore>,
    orientation: Orientation,
) -> Result<CaptionProfile, String> {
    let settings = settings::get_settings(&app);
    let project_set = project_profiles(&project);
    let set = project_set.unwrap_or(settings.caption_profiles);
    Ok(select(&set, orientation))
}

/// Write a caption profile to either the app settings or the currently
/// open project. Slice B R-005 / AC-005-c / AC-005-d.
#[tauri::command]
#[specta::specta]
pub fn set_caption_profile(
    app: AppHandle,
    project: State<CurrentProjectStore>,
    orientation: Orientation,
    profile: CaptionProfile,
    scope: ProfileScope,
) -> Result<(), String> {
    match scope {
        ProfileScope::App => {
            let mut settings = settings::get_settings(&app);
            write(&mut settings.caption_profiles, orientation, profile);
            settings.caption_profiles_was_migrated = true;
            settings::write_settings(&app, settings);
            Ok(())
        }
        ProfileScope::Project => {
            let mut guard = project
                .0
                .lock()
                .map_err(|e| format!("project lock poisoned: {e}"))?;
            let mut set = guard.clone().unwrap_or_else(|| {
                let settings = settings::get_settings(&app);
                settings.caption_profiles
            });
            write(&mut set, orientation, profile);
            *guard = Some(set);
            Ok(())
        }
    }
}

/// Compute the authoritative [`CaptionLayout`] for the given orientation
/// + frame dimensions. Both the live-preview React code and the libass
/// export composer invoke this same function (directly here, or via
/// `CaptionLayoutConfig::from_profile` in the export path), so results
/// are byte-identical. Slice B R-004-a / AC-004-b.
#[tauri::command]
#[specta::specta]
pub fn get_caption_layout(
    app: AppHandle,
    project: State<CurrentProjectStore>,
    orientation: Orientation,
    video_dims: VideoDims,
) -> Result<CaptionLayout, String> {
    let profile = get_caption_profile(app, project, orientation)?;
    Ok(compute_caption_layout(&profile, video_dims))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{default_desktop_profile, default_mobile_profile, get_default_settings};

    fn set() -> CaptionProfileSet {
        CaptionProfileSet {
            desktop: default_desktop_profile(),
            mobile: default_mobile_profile(),
        }
    }

    #[test]
    fn get_caption_profile_returns_project_when_present() {
        // Pure-logic test (no AppHandle required): the project profile
        // set wins over the app profile set when the project store is
        // populated. Mirrors the behavior of `get_caption_profile`.
        let mut project_set = set();
        project_set.desktop.font_size = 99;

        let app_set = set();
        let effective = match Some(&project_set) {
            Some(p) => select(p, Orientation::Desktop),
            None => select(&app_set, Orientation::Desktop),
        };
        assert_eq!(effective.font_size, 99);
    }

    #[test]
    fn get_caption_profile_falls_back_to_app_when_project_is_none() {
        let app_set = set();
        let project_set: Option<&CaptionProfileSet> = None;
        let effective = match project_set {
            Some(p) => select(p, Orientation::Mobile),
            None => select(&app_set, Orientation::Mobile),
        };
        assert_eq!(effective.font_size, default_mobile_profile().font_size);
    }

    #[test]
    fn set_caption_profile_app_scope_persists_to_app_settings() {
        // Drives the AppSettings mutation path that
        // `set_caption_profile(scope=App)` performs, without spinning
        // up a tauri store.
        let mut settings = get_default_settings();
        let mut new_profile = default_desktop_profile();
        new_profile.font_size = 77;

        write(
            &mut settings.caption_profiles,
            Orientation::Desktop,
            new_profile,
        );
        settings.caption_profiles_was_migrated = true;

        assert_eq!(settings.caption_profiles.desktop.font_size, 77);
        assert!(settings.caption_profiles_was_migrated);
    }

    #[test]
    fn set_caption_profile_project_scope_persists_to_open_project() {
        // Drives the project-store mutation path that
        // `set_caption_profile(scope=Project)` performs.
        let mut project_set: Option<CaptionProfileSet> = None;

        let app_settings = get_default_settings();
        let mut new_profile = default_mobile_profile();
        new_profile.position = 42;

        let mut working = project_set
            .clone()
            .unwrap_or_else(|| app_settings.caption_profiles.clone());
        write(&mut working, Orientation::Mobile, new_profile);
        project_set = Some(working);

        let saved = project_set.unwrap();
        assert_eq!(saved.mobile.position, 42);
    }
}
