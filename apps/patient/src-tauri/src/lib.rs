//! Monobase Patient - Cross-platform healthcare patient portal
//!
//! Both desktop and mobile use the same embedded Boa JS engine with Hono backend
//! for offline-first operation. Desktop additionally has tray icon, updater, and
//! single-instance support.

use tauri::Manager;
use serde_json::json;

// ===== Desktop-only modules =====
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub mod tray;

// ===== Cross-platform modules (embedded backend) =====
pub mod mobile;
pub mod engine;
pub mod db;
pub mod sync;  // Sync engine (stubbed until Cadence is added to Monobase)

// ===== Desktop-only imports =====
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use tauri::{Emitter, WindowEvent};
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use tauri_plugin_updater::UpdaterExt;
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

// ===== Cross-platform Commands =====

/// Get runtime configuration for API endpoints
///
/// Both desktop and mobile use the embedded local backend via IPC.
#[tauri::command]
async fn get_runtime_config() -> Result<serde_json::Value, String> {
    Ok(json!({
        "api_url": "tauri://localhost",
        "mode": "local",
    }))
}

// ===== Desktop-only Functions =====

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub async fn check_for_updates<R: tauri::Runtime>(handle: tauri::AppHandle<R>, is_manual: bool) {
    if let Ok(updater) = handle.updater() {
        match updater.check().await {
            Ok(Some(update)) => {
                println!("Update available: {} -> {}", update.current_version, update.version);

                let message = format!(
                    "Update v{} is available. Would you like to install it now?",
                    update.version
                );

                let confirm = handle
                    .dialog()
                    .message(message)
                    .title("Update Available")
                    .kind(MessageDialogKind::Info)
                    .buttons(MessageDialogButtons::YesNo)
                    .blocking_show();

                if confirm {
                    println!("User confirmed update, downloading and installing...");

                    use std::sync::{Arc, Mutex};
                    let downloaded = Arc::new(Mutex::new(0u64));
                    let last_percentage = Arc::new(Mutex::new(0u8));
                    let handle_for_progress = handle.clone();

                    if let Err(e) = update.download_and_install(
                        move |chunk_length, content_length| {
                            let mut downloaded_bytes = downloaded.lock().unwrap();
                            *downloaded_bytes += chunk_length as u64;

                            if let Some(total) = content_length {
                                let percentage = ((*downloaded_bytes as f64 / total as f64) * 100.0) as u8;
                                let mut last_pct = last_percentage.lock().unwrap();

                                if percentage >= *last_pct + 5 || percentage == 100 {
                                    *last_pct = percentage;
                                    println!("Download progress: {}% ({}/{})", percentage, *downloaded_bytes, total);

                                    let _ = handle_for_progress.emit("update-download-progress", json!({
                                        "percentage": percentage,
                                        "downloaded": *downloaded_bytes,
                                        "total": total
                                    }));
                                }
                            } else {
                                println!("Downloaded: {} bytes", *downloaded_bytes);
                            }
                        },
                        || {
                            println!("Update download completed, installing...");
                        }
                    ).await {
                        eprintln!("Failed to download and install update: {}", e);
                        handle
                            .dialog()
                            .message(format!("Failed to install update: {}", e))
                            .title("Update Error")
                            .kind(MessageDialogKind::Error)
                            .blocking_show();
                    } else {
                        println!("Update installed successfully, preparing to restart...");

                        handle
                            .dialog()
                            .message("Update downloaded and installed successfully. The application will restart to apply the update.")
                            .title("Update Complete")
                            .kind(MessageDialogKind::Info)
                            .buttons(MessageDialogButtons::Ok)
                            .blocking_show();

                        println!("Restarting application...");
                        handle.restart();
                    }
                } else {
                    println!("User declined update");
                }
            }
            Ok(None) => {
                println!("No updates available");
                if is_manual {
                    let current_version = handle.package_info().version.to_string();
                    handle
                        .dialog()
                        .message(format!("You're running the latest version (v{}).", current_version))
                        .title("No Updates Available")
                        .kind(MessageDialogKind::Info)
                        .buttons(MessageDialogButtons::Ok)
                        .blocking_show();
                }
            }
            Err(e) => {
                eprintln!("Failed to check for updates: {}", e);
                if is_manual {
                    handle
                        .dialog()
                        .message(format!("Failed to check for updates: {}", e))
                        .title("Update Check Error")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                }
            }
        }
    }
}

// ===== Main Entry Point =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // ===== Cross-platform plugins =====
    builder = builder
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init());

    // ===== Desktop-only plugins =====
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.unminimize();
                }
            }))
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    // ===== Desktop-only window close handler =====
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        builder = builder.on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();

                let window_clone = window.clone();

                tauri::async_runtime::spawn(async move {
                    let confirm = window_clone
                        .dialog()
                        .message("Are you sure you want to close Monobase Patient?")
                        .title("Confirm Exit")
                        .kind(MessageDialogKind::Warning)
                        .buttons(MessageDialogButtons::YesNo)
                        .blocking_show();

                    if confirm {
                        std::process::exit(0);
                    }
                });
            }
            _ => {}
        });
    }

    // ===== Setup =====
    builder = builder.setup(|app| {
        // Initialize log plugin with proper devtools integration
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            let (tauri_plugin_log, max_level, logger) =
                tauri_plugin_log::Builder::default().split(app.handle())?;

            let mut devtools_builder = tauri_plugin_devtools::Builder::default();
            devtools_builder.attach_logger(logger);
            app.handle().plugin(devtools_builder.init())?;

            app.handle().plugin(tauri_plugin_log)?;
            let _ = max_level;
        }

        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            if let Err(e) = app.handle().plugin(tauri_plugin_log::Builder::default().build()) {
                eprintln!("Failed to initialize log plugin: {} - continuing without logging", e);
            }
        }

        // Start embedded backend on ALL platforms
        if let Err(e) = mobile::setup_mobile(app) {
            eprintln!("Failed to setup embedded backend: {} - continuing with degraded functionality", e);
        }

        // Initialize sync engine (stubbed until Cadence is added to Monobase)
        {
            let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("./data"));
            let db_path = app_data_dir.join("patient.db").to_string_lossy().to_string();
            let handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                match sync::SyncState::init(&db_path).await {
                    Ok(sync_state) => {
                        if let Err(e) = sync_state.start().await {
                            log::warn!("[Sync] Failed to auto-start sync engine: {}", e);
                        }
                        handle.manage(sync_state);
                        log::info!("[Sync] Sync engine initialized and managed by Tauri");
                    }
                    Err(e) => {
                        log::error!("[Sync] Failed to initialize sync state: {}", e);
                    }
                }
            });
        }

        // Desktop-specific setup (tray, updater, signal handling)
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            setup_desktop(app)?;
        }

        Ok(())
    });

    // ===== Commands (same on all platforms now) =====
    builder = builder.invoke_handler(tauri::generate_handler![
        get_runtime_config,
        mobile::api_request,
        mobile::get_backend_status,
        // Sync commands (stubbed until Cadence is added)
        sync::sync_get_status,
        sync::sync_start,
        sync::sync_stop,
        sync::sync_configure,
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ===== Desktop Setup =====

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn setup_desktop(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // Signal handling for graceful shutdown
    tauri::async_runtime::spawn(async move {
        use tokio::signal;

        let shutdown_signal = async {
            let ctrl_c = async {
                signal::ctrl_c().await
                    .expect("failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {
                    println!("Received Ctrl+C signal");
                },
                _ = terminate => {
                    println!("Received terminate signal");
                },
            }
        };

        shutdown_signal.await;
        println!("Graceful shutdown initiated");
        std::process::exit(0);
    });

    // Setup system tray
    tray::setup_system_tray(&app_handle)?;

    // Check for updates on startup (in background)
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        check_for_updates(handle, false).await;
    });

    Ok(())
}
