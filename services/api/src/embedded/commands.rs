//! Tauri IPC commands — bridge between frontend and HapihubEmbedded.

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::transport::ipc::{IpcRequest, IpcResponse};
use super::HapihubEmbedded;

/// Shared state type for Tauri
pub type HapihubState = Arc<Mutex<Option<HapihubEmbedded>>>;

/// Create default (empty) state for Tauri app builder
pub fn default_state() -> HapihubState {
    Arc::new(Mutex::new(None))
}

/// Initialize hapihub with a SQLite database path.
/// Call once during app startup.
pub async fn hapihub_init(
    db_path: String,
    state: &HapihubState,
) -> Result<(), String> {
    let hapihub = HapihubEmbedded::new(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    *state.lock().await = Some(hapihub);
    Ok(())
}

/// Main request handler — accepts IpcRequest, returns IpcResponse.
/// NO HTTP parsing, NO network — direct service call.
pub async fn hapihub_request(
    request: IpcRequest,
    state: &HapihubState,
) -> Result<IpcResponse, String> {
    let guard = state.lock().await;
    let hapihub = guard.as_ref()
        .ok_or("Hapihub not initialized")?;

    Ok(hapihub.request(request).await)
}

/// Force WAL checkpoint — call from iOS lifecycle events
/// (applicationWillResignActive) to prevent corruption.
pub async fn hapihub_checkpoint(
    state: &HapihubState,
) -> Result<(), String> {
    let guard = state.lock().await;
    let hapihub = guard.as_ref()
        .ok_or("Hapihub not initialized")?;

    hapihub.checkpoint().await.map_err(|e| e.to_string())
}

/// Graceful shutdown — checkpoint and close connections.
pub async fn hapihub_shutdown(
    state: &HapihubState,
) -> Result<(), String> {
    let mut guard = state.lock().await;
    if let Some(hapihub) = guard.take() {
        hapihub.checkpoint().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
