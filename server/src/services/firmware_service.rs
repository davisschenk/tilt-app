use std::path::{Path, PathBuf};

use rocket::tokio::fs;
use shared::FirmwareManifest;

/// Directory holding `manifest.json` and the firmware images it lists.
#[derive(Debug, Clone)]
pub struct FirmwareDir(pub PathBuf);

impl FirmwareDir {
    pub fn from_env() -> Self {
        let dir = std::env::var("FIRMWARE_DIR")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "/app/firmware".to_string());
        Self(PathBuf::from(dir))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug)]
pub enum FirmwareError {
    NotAvailable,
    Invalid(String),
}

/// Read `manifest.json` from disk. Re-read on every call so images can be
/// swapped (e.g. via a mounted volume) without restarting the server.
pub async fn load_manifest(dir: &FirmwareDir) -> Result<FirmwareManifest, FirmwareError> {
    let raw = fs::read_to_string(dir.path().join("manifest.json"))
        .await
        .map_err(|_| FirmwareError::NotAvailable)?;
    serde_json::from_str(&raw).map_err(|e| FirmwareError::Invalid(e.to_string()))
}

/// Resolve `name` to a file on disk, but only if the manifest lists it.
/// Anything else (including path traversal attempts) yields `None`.
pub fn resolve_file(dir: &FirmwareDir, manifest: &FirmwareManifest, name: &str) -> Option<PathBuf> {
    let listed = name == manifest.files.merged.name || name == manifest.files.app.name;
    if listed {
        Some(dir.path().join(name))
    } else {
        None
    }
}
