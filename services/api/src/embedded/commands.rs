//! Tauri IPC command handlers for embedded mode.
//!
//! These functions are designed to be registered as Tauri commands:
//! ```rust
//! tauri::Builder::default()
//!     .invoke_handler(tauri::generate_handler![
//!         monobase_init,
//!         monobase_request,
//!         monobase_checkpoint,
//!         monobase_shutdown,
//!     ])
//! ```

use std::sync::Arc;
use tokio::sync::Mutex;

use super::runtime::MonobaseEmbedded;
use crate::transport::ipc::{IpcRequest, IpcResponse};

/// Shared state for Tauri commands.
pub type MonobaseState = Arc<Mutex<Option<MonobaseEmbedded>>>;

/// Initialize the embedded Monobase instance.
pub async fn monobase_init(
    db_path: &str,
    state: &MonobaseState,
) -> Result<(), String> {
    let instance = MonobaseEmbedded::new(db_path)
        .await
        .map_err(|e| format!("Init failed: {}", e))?;

    let mut guard = state.lock().await;
    *guard = Some(instance);

    Ok(())
}

/// Handle an IPC request.
pub async fn monobase_request(
    request: IpcRequest,
    state: &MonobaseState,
) -> Result<IpcResponse, String> {
    let guard = state.lock().await;
    let instance = guard
        .as_ref()
        .ok_or_else(|| "Monobase not initialized".to_string())?;

    Ok(instance.request(request).await)
}

/// Force WAL checkpoint — call on iOS lifecycle events.
pub async fn monobase_checkpoint(state: &MonobaseState) -> Result<(), String> {
    let guard = state.lock().await;
    let instance = guard
        .as_ref()
        .ok_or_else(|| "Monobase not initialized".to_string())?;

    instance
        .checkpoint()
        .await
        .map_err(|e| format!("Checkpoint failed: {}", e))
}

/// Graceful shutdown.
pub async fn monobase_shutdown(state: &MonobaseState) -> Result<(), String> {
    let mut guard = state.lock().await;
    *guard = None;
    Ok(())
}
