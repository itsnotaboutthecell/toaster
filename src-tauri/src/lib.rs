pub mod audio_toolkit;
pub mod cli;
pub mod commands;
pub mod lock_recovery;
pub mod managers;
pub mod portable;
pub mod settings;
mod utils;

pub use cli::CliArgs;
#[cfg(debug_assertions)]
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri_specta::{collect_commands, collect_events, Builder};

use commands::editor::EditorStore;
use env_filter::Builder as EnvFilterBuilder;
use managers::editor::EditorState;
use managers::media::{MediaState, MediaStore};
use managers::model::ModelManager;
use managers::transcription::TranscriptionManager;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_log::{Builder as LogBuilder, RotationStrategy, Target, TargetKind};

use crate::settings::get_settings;

// Global atomic to store the file log level filter
// We use u8 to store the log::LevelFilter as a number
pub static FILE_LOG_LEVEL: AtomicU8 = AtomicU8::new(log::LevelFilter::Debug as u8);

fn level_filter_from_u8(value: u8) -> log::LevelFilter {
    match value {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Trace,
    }
}

fn build_console_filter() -> env_filter::Filter {
    let mut builder = EnvFilterBuilder::new();

    match std::env::var("RUST_LOG") {
        Ok(spec) if !spec.trim().is_empty() => {
            if let Err(err) = builder.try_parse(&spec) {
                log::warn!(
                    "Ignoring invalid RUST_LOG value '{}': {}. Falling back to info-level console logging",
                    spec,
                    err
                );
                builder.filter_level(log::LevelFilter::Info);
            }
        }
        _ => {
            builder.filter_level(log::LevelFilter::Info);
        }
    }

    builder.build()
}

fn show_main_window(app: &AppHandle) {
    if let Some(main_window) = app.get_webview_window("main") {
        if let Err(e) = main_window.unminimize() {
            log::error!("Failed to unminimize webview window: {}", e);
        }
        if let Err(e) = main_window.show() {
            log::error!("Failed to show webview window: {}", e);
        }
        if let Err(e) = main_window.set_focus() {
            log::error!("Failed to focus webview window: {}", e);
        }
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = app.set_activation_policy(tauri::ActivationPolicy::Regular) {
                log::error!("Failed to set activation policy to Regular: {}", e);
            }
        }
        return;
    }

    let webview_labels = app.webview_windows().keys().cloned().collect::<Vec<_>>();
    log::error!(
        "Main window not found. Webview labels: {:?}",
        webview_labels
    );
}

#[allow(unused_variables)]
fn should_force_show_permissions_window(app: &AppHandle) -> bool {
    #[cfg(target_os = "windows")]
    {
        let model_manager = app.state::<Arc<ModelManager>>();
        let has_downloaded_models = model_manager
            .get_available_models()
            .iter()
            .any(|model| model.is_downloaded);

        if !has_downloaded_models {
            return false;
        }

        let status = commands::audio::get_windows_microphone_permission_status();
        if status.supported && status.overall_access == commands::audio::PermissionAccess::Denied {
            log::info!(
                "Windows microphone permissions are denied; forcing main window visible for onboarding"
            );
            return true;
        }
    }

    false
}

fn initialize_core_logic(app_handle: &AppHandle) {
    // Note: Enigo (keyboard/mouse simulation) is NOT initialized here.
    // The frontend is responsible for calling the `initialize_enigo` command
    // after onboarding completes. This avoids triggering permission dialogs
    // on macOS before the user is ready.

    // Initialize the managers
    let model_manager =
        Arc::new(ModelManager::new(app_handle).expect("Failed to initialize model manager"));
    let transcription_manager = Arc::new(
        TranscriptionManager::new(app_handle, model_manager.clone())
            .expect("Failed to initialize transcription manager"),
    );

    // Local LLM manager removed in R9 (post-processor purge).

    // Apply accelerator preferences before any model loads
    managers::transcription::apply_accelerator_settings(app_handle);

    // Add managers to Tauri's managed state
    app_handle.manage(model_manager.clone());
    app_handle.manage(transcription_manager.clone());
    app_handle.manage(EditorStore(Mutex::new(EditorState::new())));
    app_handle.manage(MediaStore(Mutex::new(MediaState::new())));
    app_handle.manage(crate::commands::project::CurrentProjectStore::default());

    // Note: Keyboard shortcuts and Unix signal handlers have been removed
    // (legacy Handy dictation surface). Toaster is a transcript editor
    // and does not register global hotkeys or signal-based toggles.

    // Apply macOS Accessory policy if starting hidden.
    #[cfg(target_os = "macos")]
    {
        let settings = settings::get_settings(app_handle);
        if settings.start_hidden {
            let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
}

#[tauri::command]
#[specta::specta]
fn trigger_update_check(app: AppHandle) -> Result<(), String> {
    let settings = settings::get_settings(&app);
    if !settings.update_checks_enabled {
        return Ok(());
    }
    app.emit("check-for-updates", ())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
fn show_main_window_command(app: AppHandle) -> Result<(), String> {
    show_main_window(&app);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(cli_args: CliArgs) {
    // Detect portable mode before anything else
    portable::init();

    // Parse console logging directives from RUST_LOG, falling back to info-level logging
    // when the variable is unset
    let console_filter = build_console_filter();

    let specta_builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_settings::change_translate_to_english_setting,
            commands::app_settings::change_selected_language_setting,
            commands::app_settings::change_debug_mode_setting,
            commands::app_settings::change_word_correction_threshold_setting,
            commands::app_settings::update_custom_words,
            commands::app_settings::change_custom_filler_words_setting,
            commands::app_settings::change_caption_font_size_setting,
            commands::app_settings::change_caption_bg_color_setting,
            commands::app_settings::change_caption_text_color_setting,
            commands::app_settings::change_caption_position_setting,
            commands::app_settings::change_caption_font_family_setting,
            commands::app_settings::change_caption_radius_px_setting,
            commands::app_settings::change_caption_padding_x_px_setting,
            commands::app_settings::change_caption_padding_y_px_setting,
            commands::app_settings::change_caption_max_width_percent_setting,
            commands::app_settings::change_lazy_stream_close_setting,
            commands::app_settings::change_normalize_audio_setting,
            commands::app_settings::change_loudness_target_setting,
            commands::app_settings::change_export_volume_db_setting,
            commands::app_settings::change_export_fade_in_ms_setting,
            commands::app_settings::change_export_fade_out_ms_setting,
            commands::app_settings::change_app_language_setting,
            commands::app_settings::change_update_checks_setting,
            commands::app_settings::change_whisper_accelerator_setting,
            commands::app_settings::change_ort_accelerator_setting,
            commands::app_settings::change_whisper_gpu_device,
            commands::app_settings::get_available_accelerators,
            trigger_update_check,
            show_main_window_command,
            commands::cancel_operation,
            commands::is_portable,
            commands::get_app_dir_path,
            commands::get_app_settings,
            commands::get_default_settings,
            commands::get_log_dir_path,
            commands::set_log_level,
            commands::open_recordings_folder,
            commands::open_log_dir,
            commands::open_app_data_dir,
            commands::models::get_available_models,
            commands::models::get_models,
            commands::models::get_model_info,
            commands::models::download_model,
            commands::models::delete_model,
            commands::models::cancel_download,
            commands::models::set_active_model,
            commands::models::get_current_model,
            commands::models::get_transcription_model_status,
            commands::models::is_model_loading,
            commands::models::has_any_models_available,
            commands::models::has_any_models_or_downloads,
            commands::audio::get_windows_microphone_permission_status,
            commands::audio::open_microphone_privacy_settings,
            commands::audio::get_available_microphones,
            commands::audio::get_available_output_devices,
            commands::audio::set_selected_output_device,
            commands::audio::get_selected_output_device,
            commands::audio::normalize_playback_audio_contract,
            commands::editor::editor_set_words,
            commands::editor::editor_get_words,
            commands::editor::editor_delete_word,
            commands::editor::editor_restore_word,
            commands::editor::editor_delete_range,
            commands::editor::editor_restore_all,
            commands::editor::editor_split_word,
            commands::editor::editor_silence_word,
            commands::editor::editor_undo,
            commands::editor::editor_redo,
            commands::editor::editor_get_keep_segments,
            commands::editor::editor_get_timing_contract,
            commands::editor::editor_get_projection,
            commands::media::media_import,
            commands::media::media_get_current,
            commands::media::media_get_asset_url,
            commands::media::media_clear,
            commands::export::export_transcript,
            commands::export::export_transcript_to_file,
            commands::export::get_caption_segments,
            commands::export::get_caption_blocks,
            commands::transcribe_file::transcribe_media_file,
            commands::waveform::generate_waveform_peaks,
            commands::waveform::get_keep_segments,
            commands::waveform::generate_ffmpeg_edit_script,
            commands::waveform::map_edit_to_source_time,
            commands::waveform::invalidate_temp_preview_cache,
            commands::waveform::render_temp_preview_audio,
            commands::waveform::export_edited_media,
            commands::waveform::list_allowed_export_formats,
            commands::waveform::loudness_preflight,
            commands::filler::analyze_fillers,
            commands::filler::delete_fillers,
            commands::filler::delete_duplicates,
            commands::filler::silence_pauses,
            commands::filler::trim_pauses,
            commands::filler::tighten_gaps,
            commands::filler::cleanup_all,
            commands::disfluency::cleanup_smart_duplicates,
            commands::project::save_project,
            commands::project::load_project,
            commands::captions::get_caption_profile,
            commands::captions::set_caption_profile,
            commands::captions::get_caption_layout,
            commands::transcription::set_model_unload_timeout,
            commands::transcription::get_model_load_status,
            commands::transcription::unload_model_manually,
        ])
        .events(collect_events![]);

    #[cfg(debug_assertions)] // <- Only export on non-release builds
    {
        const BINDINGS_PATH: &str = "../src/bindings.ts";
        specta_builder
            .export(
                Typescript::default().bigint(BigIntExportBehavior::Number),
                BINDINGS_PATH,
            )
            .expect("Failed to export typescript bindings");

        // Post-codegen: tauri-specta 2.0.0-rc.21 hardcodes `e as any` in the
        // generated catch branches (see specta/tauri-specta#? — the flag is
        // always true when the command returns a Result). Rewrite those
        // sites to `e as string`, which matches every command's declared
        // `Result<T, string>` error type and removes ~90 `any`s from the
        // generated file. Frontend TS strict mode enforces this contract.
        // TODO: drop this shim when upgrading tauri-specta to a release
        // that lets us configure the cast target directly.
        if let Ok(src) = std::fs::read_to_string(BINDINGS_PATH) {
            let mut patched = src.replace("e  as any", "e as string");
            // When `collect_events![]` is empty, tauri-specta still emits the
            // `Channel as TAURI_CHANNEL` import and the `__makeEvents__`
            // helper, neither of which is referenced. TS strict mode then
            // fails with TS6133 ("declared but never read") and breaks
            // `bun run build`. Append a void-reference trailer so both
            // identifiers count as used until events are added back. Idempotent
            // — skipped once the trailer is already present.
            const VOID_TRAILER: &str =
                "\nvoid TAURI_CHANNEL;\nvoid __makeEvents__;\n";
            if !patched.contains("void TAURI_CHANNEL;") {
                patched.push_str(VOID_TRAILER);
            }
            if patched != src {
                if let Err(err) = std::fs::write(BINDINGS_PATH, patched) {
                    eprintln!("Failed to post-process bindings.ts: {err}");
                }
            }
        }
    }

    let invoke_handler = specta_builder.invoke_handler();

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            LogBuilder::new()
                .level(log::LevelFilter::Trace) // Set to most verbose level globally
                .max_file_size(500_000)
                .rotation_strategy(RotationStrategy::KeepOne)
                .clear_targets()
                .targets([
                    // Console output respects RUST_LOG environment variable
                    Target::new(TargetKind::Stdout).filter({
                        let console_filter = console_filter.clone();
                        move |metadata| console_filter.enabled(metadata)
                    }),
                    // File logs respect the user's settings (stored in FILE_LOG_LEVEL atomic)
                    Target::new(if let Some(data_dir) = portable::data_dir() {
                        TargetKind::Folder {
                            path: data_dir.join("logs"),
                            file_name: Some("toaster".into()),
                        }
                    } else {
                        TargetKind::LogDir {
                            file_name: Some("toaster".into()),
                        }
                    })
                    .filter(|metadata| {
                        let file_level = FILE_LOG_LEVEL.load(Ordering::Relaxed);
                        metadata.level() <= level_filter_from_u8(file_level)
                    }),
                ])
                .build(),
        );

    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(cli_args.clone())
        .setup(move |app| {
            specta_builder.mount_events(app);

            // Create main window programmatically so we can set data_directory
            // for portable mode (redirects WebView2 cache to portable Data dir)
            let mut win_builder =
                tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("/".into()))
                    .title("Toaster")
                    .inner_size(680.0, 570.0)
                    .min_inner_size(680.0, 570.0)
                    .resizable(true)
                    .maximizable(false)
                    .visible(false);

            if let Some(data_dir) = portable::data_dir() {
                win_builder = win_builder.data_directory(data_dir.join("webview"));
            }

            win_builder.build()?;

            let mut settings = get_settings(app.handle());

            // CLI --debug flag overrides debug_mode and log level (runtime-only, not persisted)
            if cli_args.debug {
                settings.debug_mode = true;
                settings.log_level = settings::LogLevel::Trace;
            }

            let tauri_log_level: tauri_plugin_log::LogLevel = settings.log_level.into();
            let file_log_level: log::Level = tauri_log_level.into();
            // Store the file log level in the atomic for the filter to use
            FILE_LOG_LEVEL.store(file_log_level.to_level_filter() as u8, Ordering::Relaxed);
            let app_handle = app.handle().clone();

            initialize_core_logic(&app_handle);

            // Pre-warm GPU/accelerator enumeration on a background thread.
            // The first call into transcribe_rs::whisper_cpp::gpu::list_gpu_devices
            // loads the Metal/Vulkan backend and probes devices, which can take
            // several seconds. Without this, that cost is paid synchronously the
            // first time the user opens the Advanced settings page (which calls
            // the get_available_accelerators command), causing a UI freeze.
            // Result is cached in a OnceLock inside the transcription manager.
            std::thread::spawn(|| {
                let _ = crate::managers::transcription::get_available_accelerators();
            });

            // Show main window only if not starting hidden.
            // CLI --start-hidden flag overrides the setting.
            // But if permission onboarding is required, always show the window.
            let should_hide = settings.start_hidden || cli_args.start_hidden;
            let should_force_show = should_force_show_permissions_window(&app_handle);

            if should_force_show || !should_hide {
                show_main_window(&app_handle);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _res = window.hide();

                #[cfg(target_os = "macos")]
                {
                    // No tray: keep the dock icon visible so the user can reopen
                    let _ = window;
                }
            }
        })
        .invoke_handler(invoke_handler)
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = &event {
                show_main_window(app);
            }
            let _ = (app, event); // suppress unused warnings on non-macOS
        });
}
