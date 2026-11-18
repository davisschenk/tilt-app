use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, get};
use shared::FirmwareManifest;

use crate::services::firmware_service::{self, FirmwareDir, FirmwareError};

type ApiError = (Status, Json<serde_json::Value>);

fn err(status: Status, msg: &str) -> ApiError {
    (status, Json(serde_json::json!({ "error": msg })))
}

fn map_err(e: FirmwareError) -> ApiError {
    match e {
        FirmwareError::NotAvailable => err(Status::NotFound, "no firmware available"),
        FirmwareError::Invalid(detail) => {
            tracing::error!("invalid firmware manifest: {detail}");
            err(Status::InternalServerError, "invalid firmware manifest")
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareInfo {
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    /// Hex Ed25519 signature; devices only install images whose signature verifies.
    pub signature: Option<String>,
}

/// Manifest of the firmware bundled with this server (used by the web flasher).
#[get("/firmware/manifest")]
pub async fn get_manifest(dir: &State<FirmwareDir>) -> Result<Json<FirmwareManifest>, ApiError> {
    firmware_service::load_manifest(dir.inner())
        .await
        .map(Json)
        .map_err(map_err)
}

/// Serve a firmware image. Only files listed in the manifest are reachable.
#[get("/firmware/files/<name>")]
pub async fn get_file(dir: &State<FirmwareDir>, name: &str) -> Result<NamedFile, ApiError> {
    let manifest = firmware_service::load_manifest(dir.inner())
        .await
        .map_err(map_err)?;
    let path = firmware_service::resolve_file(dir.inner(), &manifest, name)
        .ok_or_else(|| err(Status::NotFound, "unknown firmware file"))?;
    NamedFile::open(path)
        .await
        .map_err(|_| err(Status::NotFound, "firmware file missing"))
}

/// OTA manifest polled by devices. Always derived from the bundled firmware manifest
/// (there is deliberately no free-form URL override: devices verify a signature that
/// only the release pipeline can produce). The URL is relative; the device resolves it
/// against its configured server URL.
#[get("/ota/firmware")]
pub async fn get_firmware(dir: &State<FirmwareDir>) -> Result<Json<FirmwareInfo>, ApiError> {
    let manifest = firmware_service::load_manifest(dir.inner())
        .await
        .map_err(map_err)?;
    Ok(Json(FirmwareInfo {
        version: manifest.version,
        url: format!("/api/v1/firmware/files/{}", manifest.files.app.name),
        sha256: manifest.files.app.sha256,
        size: manifest.files.app.size,
        signature: manifest.signature,
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![get_manifest, get_file, get_firmware]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::blocking::Client;
    use std::path::PathBuf;

    fn fixture() -> (Client, PathBuf) {
        let dir = std::env::temp_dir().join(format!("tilt-fw-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app.bin"), b"app-bytes").unwrap();
        std::fs::write(dir.join("merged.bin"), b"merged-bytes").unwrap();
        std::fs::write(dir.join("secret.txt"), b"nope").unwrap();
        let manifest = r#"{"version":"1.2.3","chip":"esp32","builtAt":"2026-01-01T00:00:00Z",
            "files":{"merged":{"name":"merged.bin","sha256":"aa","size":12},
                     "app":{"name":"app.bin","sha256":"bb","size":9}},
            "signature":"cc"}"#;
        std::fs::write(dir.join("manifest.json"), manifest).unwrap();
        let rocket = rocket::build()
            .manage(FirmwareDir(dir.clone()))
            .mount("/api/v1", routes());
        (Client::tracked(rocket).unwrap(), dir)
    }

    #[test]
    fn manifest_is_returned() {
        let (client, dir) = fixture();
        let resp = client.get("/api/v1/firmware/manifest").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = resp.into_json().unwrap();
        assert_eq!(body["version"], "1.2.3");
        assert_eq!(body["files"]["app"]["name"], "app.bin");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn only_listed_files_are_served() {
        let (client, dir) = fixture();
        let ok = client.get("/api/v1/firmware/files/app.bin").dispatch();
        assert_eq!(ok.status(), Status::Ok);
        assert_eq!(ok.into_bytes().unwrap(), b"app-bytes");
        let unlisted = client.get("/api/v1/firmware/files/secret.txt").dispatch();
        assert_eq!(unlisted.status(), Status::NotFound);
        let traversal = client
            .get("/api/v1/firmware/files/..%2Fsecret.txt")
            .dispatch();
        assert_ne!(traversal.status(), Status::Ok);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ota_advertises_signed_app_image_with_relative_url() {
        let (client, dir) = fixture();
        let resp = client.get("/api/v1/ota/firmware").dispatch();
        assert_eq!(resp.status(), Status::Ok);
        let body: serde_json::Value = resp.into_json().unwrap();
        assert_eq!(body["version"], "1.2.3");
        assert_eq!(body["url"], "/api/v1/firmware/files/app.bin");
        assert_eq!(body["sha256"], "bb");
        assert_eq!(body["size"], 9);
        assert_eq!(body["signature"], "cc");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_manifest_is_404() {
        let rocket = rocket::build()
            .manage(FirmwareDir(PathBuf::from("/nonexistent-firmware-dir")))
            .mount("/api/v1", routes());
        let client = Client::tracked(rocket).unwrap();
        let resp = client.get("/api/v1/firmware/manifest").dispatch();
        assert_eq!(resp.status(), Status::NotFound);
    }
}
