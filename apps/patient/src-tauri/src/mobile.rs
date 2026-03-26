//! Mobile-specific setup for iOS/iPadOS with embedded Hono backend
//!
//! This module sets up the mobile version of the patient app with an embedded
//! Boa JS engine running a Hono HTTP server for offline-first operation.
//! Requests are routed through IPC as HTTP method + path + body + headers.

use tauri::{App, Manager};
use std::sync::Mutex;

use crate::engine::{EngineResponse, JsEngine};

/// Managed state for the JS engine
pub struct EngineState {
    pub engine: Box<dyn JsEngine>,
}

/// Store initialization error for diagnostics
pub struct InitErrorState(pub std::sync::Arc<Mutex<Option<String>>>);

/// Handle API requests from the frontend via Tauri IPC
///
/// This replaces the old HTTP-based API with an IPC interface.
/// The frontend sends method/path/body/headers and gets back a full HTTP response.
#[tauri::command]
pub fn api_request(
    method: String,
    path: String,
    body: Option<String>,
    headers: Option<std::collections::HashMap<String, String>>,
    state: tauri::State<'_, Mutex<EngineState>>,
) -> Result<EngineResponse, String> {
    let url = format!("http://localhost{}", path);
    let header_pairs: Vec<(String, String)> = headers
        .unwrap_or_default()
        .into_iter()
        .collect();
    let header_refs: Vec<(&str, &str)> = header_pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let state = state.lock().map_err(|e| e.to_string())?;
    state.engine.handle_request(&method, &url, body.as_deref(), header_refs)
}

/// Get backend initialization status and any error
#[tauri::command]
pub fn get_backend_status(
    error_state: tauri::State<'_, InitErrorState>,
) -> Result<serde_json::Value, String> {
    let error = {
        let guard = error_state.0.lock().map_err(|e| format!("Lock error: {}", e))?;
        guard.clone()
    };

    Ok(serde_json::json!({
        "initialized": error.is_none(),
        "error": error
    }))
}

/// Set up the mobile application with embedded Hono backend
///
/// On iOS, we run an embedded Boa JS engine with a Hono HTTP server
/// and plain SQLite storage. The frontend communicates via IPC using
/// HTTP-style requests (method + path + body + headers).
///
/// This function is designed to NEVER crash the app - all errors are logged
/// and the app continues with degraded functionality.
pub fn setup_mobile(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Monobase Patient initializing in local mode...");

    // Initialize error state
    let error_state = InitErrorState(std::sync::Arc::new(Mutex::new(None)));

    // Get the app data directory for the database
    let app_data_dir = match app.path().app_data_dir() {
        Ok(dir) => {
            log::info!("App data dir: {:?}", dir);
            dir
        }
        Err(e) => {
            let err_msg = format!("Failed to get app data dir: {}", e);
            log::error!("{}", err_msg);
            if let Ok(mut guard) = error_state.0.lock() {
                *guard = Some(err_msg);
            }
            std::path::PathBuf::from("./data")
        }
    };

    // Create the directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&app_data_dir) {
        let err_msg = format!("Failed to create app data dir: {}", e);
        log::error!("{}", err_msg);
        if let Ok(mut guard) = error_state.0.lock() {
            *guard = Some(err_msg);
        }
    }

    let db_path = app_data_dir.join("patient.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    log::info!("Database path: {}", db_path_str);

    // Initialize the Boa engine with Hono backend
    use crate::engine::boa::BoaEngine;
    match BoaEngine::new(&db_path_str) {
        Ok(engine) => {
            log::info!("Boa engine initialized successfully");

            let state = Mutex::new(EngineState {
                engine: Box::new(engine),
            });
            app.manage(state);
            app.manage(error_state);

            log::info!("Monobase Patient ready in local mode");
        }
        Err(e) => {
            let err_msg = format!("Boa engine initialization failed: {}", e);
            log::error!("{}", err_msg);

            // Store the error
            if let Ok(mut guard) = error_state.0.lock() {
                *guard = Some(err_msg);
            }

            // Store a dummy engine state so the app can still launch
            // The error will be shown in the UI via get_backend_status
            app.manage(error_state);

            log::warn!("Monobase Patient starting with degraded functionality");
        }
    }

    Ok(())
}
