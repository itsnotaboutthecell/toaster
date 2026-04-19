//! Tauri store I/O: read/write the app settings store and expose convenience
//! accessors for hot-path fields.

use super::defaults::{ensure_caption_defaults, get_default_settings};
use super::types::AppSettings;
use super::SETTINGS_STORE_PATH;
use log::warn;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub fn get_settings(app: &AppHandle) -> AppSettings {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    let mut settings = if let Some(settings_value) = store.get("settings") {
        serde_json::from_value::<AppSettings>(settings_value).unwrap_or_else(|_| {
            let default_settings = get_default_settings();
            match serde_json::to_value(&default_settings) {
                Ok(val) => store.set("settings", val),
                Err(e) => warn!("Failed to serialize default settings: {}", e),
            }
            default_settings
        })
    } else {
        let default_settings = get_default_settings();
        match serde_json::to_value(&default_settings) {
            Ok(val) => store.set("settings", val),
            Err(e) => warn!("Failed to serialize default settings: {}", e),
        }
        default_settings
    };

    if ensure_caption_defaults(&mut settings) {
        match serde_json::to_value(&settings) {
            Ok(val) => store.set("settings", val),
            Err(e) => warn!("Failed to serialize settings after default migration: {}", e),
        }
    }

    settings
}

pub fn write_settings(app: &AppHandle, settings: AppSettings) {
    let store = app
        .store(crate::portable::store_path(SETTINGS_STORE_PATH))
        .expect("Failed to initialize store");

    match serde_json::to_value(&settings) {
        Ok(val) => store.set("settings", val),
        Err(e) => warn!("Failed to serialize settings for write: {}", e),
    }
}
