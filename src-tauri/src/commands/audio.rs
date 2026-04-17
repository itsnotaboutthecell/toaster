use crate::audio_toolkit::audio::{list_input_devices, list_output_devices};
use crate::settings::{get_settings, write_settings};
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use winreg::{
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    RegKey, HKEY,
};

#[derive(Serialize, Type)]
pub struct CustomSounds {
    start: bool,
    stop: bool,
}

fn custom_sound_exists(app: &AppHandle, sound_type: &str) -> bool {
    crate::portable::resolve_app_data(app, &format!("custom_{}.wav", sound_type))
        .is_ok_and(|path| path.exists())
}

#[tauri::command]
#[specta::specta]
pub fn check_custom_sounds(app: AppHandle) -> CustomSounds {
    CustomSounds {
        start: custom_sound_exists(&app, "start"),
        stop: custom_sound_exists(&app, "stop"),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct AudioDevice {
    pub index: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct PlaybackAudioContract {
    pub selected_output_device: String,
    pub selected_output_device_available: bool,
    pub preferred_output_sample_rate: u32,
    pub detected_output_sample_rate: Option<u32>,
    pub normalized_output_sample_rate: u32,
    pub mismatch_detected: bool,
}

fn normalize_selected_output_device(raw: Option<String>) -> Option<String> {
    let trimmed = raw.unwrap_or_default().trim().to_string();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        None
    } else {
        Some(trimmed)
    }
}

fn clamp_sample_rate(sample_rate: u32) -> u32 {
    sample_rate.clamp(8_000, 192_000)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAccess {
    Allowed,
    Denied,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct WindowsMicrophonePermissionStatus {
    pub supported: bool,
    pub overall_access: PermissionAccess,
    pub device_access: PermissionAccess,
    pub app_access: PermissionAccess,
    pub desktop_app_access: PermissionAccess,
}

#[cfg(target_os = "windows")]
fn read_registry_permission_access(root_hkey: HKEY, path: &str) -> PermissionAccess {
    let root = RegKey::predef(root_hkey);
    let Ok(key) = root.open_subkey(path) else {
        return PermissionAccess::Unknown;
    };

    let Ok(value) = key.get_value::<String, _>("Value") else {
        return PermissionAccess::Unknown;
    };

    match value.to_ascii_lowercase().as_str() {
        "allow" => PermissionAccess::Allowed,
        "deny" => PermissionAccess::Denied,
        _ => PermissionAccess::Unknown,
    }
}

#[cfg(target_os = "windows")]
fn get_windows_microphone_permission_status_impl() -> WindowsMicrophonePermissionStatus {
    const MICROPHONE_PATH: &str =
        "Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone";
    const DESKTOP_APPS_PATH: &str =
        "Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\microphone\\NonPackaged";

    let device_access = read_registry_permission_access(HKEY_LOCAL_MACHINE, MICROPHONE_PATH);
    let app_access = read_registry_permission_access(HKEY_CURRENT_USER, MICROPHONE_PATH);
    let desktop_app_access = read_registry_permission_access(HKEY_CURRENT_USER, DESKTOP_APPS_PATH);

    let overall_access = if [device_access, app_access, desktop_app_access]
        .into_iter()
        .any(|access| access == PermissionAccess::Denied)
    {
        PermissionAccess::Denied
    } else if [device_access, app_access, desktop_app_access]
        .into_iter()
        .all(|access| access == PermissionAccess::Allowed)
    {
        PermissionAccess::Allowed
    } else {
        PermissionAccess::Unknown
    };

    WindowsMicrophonePermissionStatus {
        supported: true,
        overall_access,
        device_access,
        app_access,
        desktop_app_access,
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_windows_microphone_permission_status() -> WindowsMicrophonePermissionStatus {
    #[cfg(target_os = "windows")]
    {
        get_windows_microphone_permission_status_impl()
    }

    #[cfg(not(target_os = "windows"))]
    {
        WindowsMicrophonePermissionStatus {
            supported: false,
            overall_access: PermissionAccess::Unknown,
            device_access: PermissionAccess::Unknown,
            app_access: PermissionAccess::Unknown,
            desktop_app_access: PermissionAccess::Unknown,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn open_microphone_privacy_settings() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("cmd")
            .args(["/C", "start", "", "ms-settings:privacy-microphone"])
            .spawn()
            .map_err(|e| format!("Failed to open Windows microphone privacy settings: {}", e))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Opening microphone privacy settings is only supported on Windows".to_string())
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_available_microphones() -> Result<Vec<AudioDevice>, String> {
    let devices =
        list_input_devices().map_err(|e| format!("Failed to list audio devices: {}", e))?;

    let mut result = vec![AudioDevice {
        index: "default".to_string(),
        name: "Default".to_string(),
        is_default: true,
    }];

    result.extend(devices.into_iter().map(|d| AudioDevice {
        index: d.index,
        name: d.name,
        is_default: false, // The explicit default is handled separately
    }));

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn get_available_output_devices() -> Result<Vec<AudioDevice>, String> {
    let devices =
        list_output_devices().map_err(|e| format!("Failed to list output devices: {}", e))?;

    let mut result = vec![AudioDevice {
        index: "default".to_string(),
        name: "Default".to_string(),
        is_default: true,
    }];

    result.extend(devices.into_iter().map(|d| AudioDevice {
        index: d.index,
        name: d.name,
        is_default: false, // The explicit default is handled separately
    }));

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn set_selected_output_device(app: AppHandle, device_name: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.selected_output_device = if device_name == "default" {
        None
    } else {
        Some(device_name)
    };
    write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_selected_output_device(app: AppHandle) -> Result<String, String> {
    let settings = get_settings(&app);
    Ok(settings
        .selected_output_device
        .unwrap_or_else(|| "default".to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn normalize_playback_audio_contract(app: AppHandle) -> Result<PlaybackAudioContract, String> {
    let mut settings = get_settings(&app);
    let requested_device =
        normalize_selected_output_device(settings.selected_output_device.clone());
    let preferred_sample_rate = clamp_sample_rate(settings.preferred_output_sample_rate);

    let host = crate::audio_toolkit::get_cpal_host();
    let mut selected_device = host.default_output_device();
    let mut selected_output_device_available = true;
    let mut selected_output_device_name = "Default".to_string();

    if let Some(requested_name) = requested_device.clone() {
        let mut found = None;
        if let Ok(devices) = host.output_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    if name == requested_name {
                        found = Some((device, name));
                        break;
                    }
                }
            }
        }

        if let Some((device, name)) = found {
            selected_device = Some(device);
            selected_output_device_name = name;
        } else {
            selected_output_device_available = false;
            settings.selected_output_device = None;
        }
    }

    let detected_output_sample_rate = selected_device.as_ref().and_then(|device| {
        device
            .default_output_config()
            .ok()
            .map(|cfg| cfg.sample_rate().0)
    });

    let normalized_output_sample_rate = detected_output_sample_rate
        .map(clamp_sample_rate)
        .unwrap_or(preferred_sample_rate);

    let mismatch_detected = normalized_output_sample_rate != preferred_sample_rate;
    if mismatch_detected || settings.preferred_output_sample_rate != preferred_sample_rate {
        settings.preferred_output_sample_rate = normalized_output_sample_rate;
    }

    if requested_device.is_none() {
        settings.selected_output_device = None;
    }

    write_settings(&app, settings);

    Ok(PlaybackAudioContract {
        selected_output_device: selected_output_device_name,
        selected_output_device_available,
        preferred_output_sample_rate: preferred_sample_rate,
        detected_output_sample_rate,
        normalized_output_sample_rate,
        mismatch_detected,
    })
}

#[cfg(test)]
mod tests {
    use super::{clamp_sample_rate, normalize_selected_output_device};

    #[test]
    fn normalize_selected_output_device_treats_default_as_none() {
        assert_eq!(normalize_selected_output_device(None), None);
        assert_eq!(
            normalize_selected_output_device(Some("default".to_string())),
            None
        );
        assert_eq!(
            normalize_selected_output_device(Some("Default".to_string())),
            None
        );
    }

    #[test]
    fn normalize_selected_output_device_preserves_named_device() {
        assert_eq!(
            normalize_selected_output_device(Some("Speakers (USB)".to_string())),
            Some("Speakers (USB)".to_string())
        );
    }

    #[test]
    fn clamp_sample_rate_bounds_values() {
        assert_eq!(clamp_sample_rate(1_000), 8_000);
        assert_eq!(clamp_sample_rate(48_000), 48_000);
        assert_eq!(clamp_sample_rate(999_999), 192_000);
    }
}
