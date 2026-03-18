use rocket::get;
use rocket::http::Status;
use rocket::serde::json::Json;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareInfo {
    pub version: String,
    pub url: String,
}

#[get("/ota/firmware")]
pub fn get_firmware() -> Result<Json<FirmwareInfo>, (Status, Json<serde_json::Value>)> {
    let version = std::env::var("OTA_FIRMWARE_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let url = std::env::var("OTA_FIRMWARE_URL").unwrap_or_default();

    if url.is_empty() {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({ "error": "no firmware available" })),
        ));
    }

    Ok(Json(FirmwareInfo { version, url }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![get_firmware]
}
