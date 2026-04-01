//! Embedded hapihub for Tauri apps.
//! Uses IPC adapter directly — no Axum, no HTTP, no network.

#[cfg(feature = "embedded")]
pub mod commands;

#[cfg(feature = "embedded")]
mod runtime;

#[cfg(feature = "embedded")]
pub use runtime::HapihubEmbedded;
