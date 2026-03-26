//! Cadence sync engine integration for Monobase Patient Portal
//!
//! Embeds the Cadence P2P sync engine in-process, sharing the same SQLite
//! database (WAL mode) as the Boa/Hono backend. Exposes Tauri commands for
//! the React frontend to control sync and monitor status.
//!
//! NOTE: Cadence integration is currently stubbed. Once Cadence is added to
//! the Monobase monorepo, uncomment the imports and implementation.

// TODO: Uncomment when cadence is added to Monobase
// use cadence::storage::runtime_backend::{Backend, StorageConfig};
// use cadence::storage::backend::StorageBackend;
// use cadence::sync::manager::SyncEngine;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Sync engine state managed by Tauri
pub struct SyncState {
    // TODO: Replace with actual SyncEngine when cadence is available
    // engine: Arc<Mutex<Option<SyncEngine>>>,
    // storage: Arc<Backend>,
    _running: Arc<Mutex<bool>>,
    peer_id: String,
    cloud_config: Mutex<Option<CloudConfig>>,
    db_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub hub_url: String,
    pub org_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncPeer {
    pub id: String,
    pub label: Option<String>,
    pub status: String,
    pub sync_status: String,
    pub last_sync_at: Option<String>,
    pub address: Option<String>,
    pub transport: String,
    pub download_progress: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatusResponse {
    pub engine_running: bool,
    pub peer_id: String,
    pub peers: Vec<SyncPeer>,
    pub cloud_configured: bool,
    pub cloud_hub_url: Option<String>,
}

impl SyncState {
    /// Initialize sync state with SQLite backend sharing the app database
    pub async fn init(db_path: &str) -> Result<Self, String> {
        // Generate a persistent peer ID
        // In production, this would be stored in the database
        let peer_id = uuid::Uuid::new_v4().to_string();

        log::info!("[Sync] Sync state initialized (stubbed), peer_id={}", peer_id);

        Ok(Self {
            _running: Arc::new(Mutex::new(false)),
            peer_id,
            cloud_config: Mutex::new(None),
            db_path: db_path.to_string(),
        })
    }

    /// Start the sync engine
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self._running.lock().await;
        if *running {
            return Ok(()); // Already running
        }

        // TODO: Initialize actual SyncEngine when cadence is available
        // let config = StorageConfig::Sqlite { path: self.db_path.clone() };
        // let storage = Backend::from_config(&config).await?;
        // let engine = SyncEngine::new(self.peer_id.clone(), Arc::new(storage));

        *running = true;
        log::info!("[Sync] Sync engine started (stubbed)");
        Ok(())
    }

    /// Stop the sync engine
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self._running.lock().await;
        *running = false;

        log::info!("[Sync] Sync engine stopped (stubbed)");
        Ok(())
    }

    /// Get current sync status
    pub async fn get_status(&self) -> SyncStatusResponse {
        let running = self._running.lock().await;
        let cloud_config = self.cloud_config.lock().await;

        SyncStatusResponse {
            engine_running: *running,
            peer_id: self.peer_id.clone(),
            peers: Vec::new(), // Populated when WebSocket connections are active
            cloud_configured: cloud_config.is_some(),
            cloud_hub_url: cloud_config.as_ref().map(|c| c.hub_url.clone()),
        }
    }

    /// Configure cloud hub connection
    pub async fn configure_cloud(&self, hub_url: String, org_id: String) -> Result<(), String> {
        let mut config = self.cloud_config.lock().await;
        *config = Some(CloudConfig {
            hub_url: hub_url.clone(),
            org_id,
        });

        log::info!("[Sync] Cloud hub configured: {}", hub_url);
        Ok(())
    }

    /// Get the database path used by sync
    pub fn db_path(&self) -> &str {
        &self.db_path
    }
}

// ===== Tauri Commands =====

#[tauri::command]
pub async fn sync_get_status(
    state: tauri::State<'_, SyncState>,
) -> Result<SyncStatusResponse, String> {
    Ok(state.get_status().await)
}

#[tauri::command]
pub async fn sync_start(
    state: tauri::State<'_, SyncState>,
) -> Result<(), String> {
    state.start().await
}

#[tauri::command]
pub async fn sync_stop(
    state: tauri::State<'_, SyncState>,
) -> Result<(), String> {
    state.stop().await
}

#[tauri::command]
pub async fn sync_configure(
    hub_url: String,
    org_id: String,
    state: tauri::State<'_, SyncState>,
) -> Result<(), String> {
    state.configure_cloud(hub_url, org_id).await
}
