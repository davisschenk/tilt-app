use super::require_brew;
use crate::guards::{auth_or_api_key::AuthOrApiKey, current_user::CurrentUser};
use crate::models::entities::{
    brew_reports as reports, brewer_preferences as prefs, event_attachments, scanners,
};
use crate::services::{brew_event_service, brew_service};
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum TemperatureUnit {
    #[default]
    F,
    C,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum GravityUnit {
    #[default]
    SG,
    P,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub temperature: TemperatureUnit,
    pub gravity: GravityUnit,
}
impl Preferences {
    pub fn format_gravity(&self, g: f64) -> String {
        match self.gravity {
            GravityUnit::SG => format!("{g:.4} SG"),
            GravityUnit::P => format!(
                "{:.2}°P",
                -616.868 + 1111.14 * g - 630.272 * g * g + 135.997 * g * g * g
            ),
        }
    }
    pub fn format_temperature(&self, t: f64, difference: bool) -> String {
        match self.temperature {
            TemperatureUnit::F => format!("{t:.1}°F"),
            TemperatureUnit::C => format!(
                "{:.1}°C",
                (t - if difference { 0.0 } else { 32.0 }) * 5.0 / 9.0
            ),
        }
    }
}
#[derive(rocket::response::Responder)]
struct NoStore<T> {
    inner: T,
    cache: rocket::http::Header<'static>,
    nosniff: rocket::http::Header<'static>,
}
fn no_store<T>(inner: T) -> NoStore<T> {
    NoStore {
        inner,
        cache: rocket::http::Header::new("Cache-Control", "no-store"),
        nosniff: rocket::http::Header::new("X-Content-Type-Options", "nosniff"),
    }
}
fn internal(_: DbErr) -> Status {
    Status::InternalServerError
}
fn uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::UnprocessableEntity)
}
pub async fn preferences(db: &DatabaseConnection, owner: Uuid) -> Result<Preferences, DbErr> {
    Ok(prefs::Entity::find_by_id(owner)
        .one(db)
        .await?
        .and_then(|r| serde_json::from_value(r.payload).ok())
        .unwrap_or_default())
}
#[get("/account/preferences")]
async fn get_preferences(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<Preferences>, Status> {
    preferences(db, user.user_id)
        .await
        .map(Json)
        .map_err(internal)
}
#[put("/account/preferences", data = "<input>")]
async fn set_preferences(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<Preferences>,
) -> Result<Json<Preferences>, Status> {
    prefs::Entity::insert(prefs::ActiveModel {
        owner_id: Set(user.user_id),
        payload: Set(json!(input.0)),
    })
    .on_conflict(
        sea_query::OnConflict::column(prefs::Column::OwnerId)
            .update_column(prefs::Column::Payload)
            .to_owned(),
    )
    .exec(db.inner())
    .await
    .map_err(internal)?;
    Ok(input)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Heartbeat {
    device_id: String,
    firmware_version: String,
    uptime_seconds: u64,
    wifi_rssi: Option<i16>,
    scan_interval_seconds: u32,
    buffered_readings: u32,
    scan_ok: bool,
    upload_ok: bool,
}
#[post("/scanners/heartbeat", data = "<input>")]
async fn heartbeat(
    auth: AuthOrApiKey,
    db: &State<DatabaseConnection>,
    input: Json<Heartbeat>,
) -> Result<Status, Status> {
    let h = input.into_inner();
    if scanners::Entity::find()
        .filter(scanners::Column::OwnerId.eq(auth.owner_id))
        .filter(scanners::Column::DeviceId.eq(&h.device_id))
        .one(db.inner())
        .await
        .map_err(internal)?
        .is_none()
        && scanners::Entity::find()
            .filter(scanners::Column::OwnerId.eq(auth.owner_id))
            .count(db.inner())
            .await
            .map_err(internal)?
            >= 32
    {
        return Err(Status::Forbidden);
    }
    if h.device_id.is_empty()
        || h.device_id.len() > 64
        || !h
            .device_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || h.firmware_version.len() > 64
        || h.scan_interval_seconds == 0
        || h.scan_interval_seconds > 86400
    {
        return Err(Status::UnprocessableEntity);
    }
    scanners::Entity::insert(scanners::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_id: Set(auth.owner_id),
        device_id: Set(h.device_id.clone()),
        payload: Set(json!(h)),
        last_seen_at: Set(chrono::Utc::now().into()),
    })
    .on_conflict(
        sea_query::OnConflict::columns([scanners::Column::OwnerId, scanners::Column::DeviceId])
            .update_columns([scanners::Column::Payload, scanners::Column::LastSeenAt])
            .to_owned(),
    )
    .exec(db.inner())
    .await
    .map_err(internal)?;
    Ok(Status::NoContent)
}
#[get("/scanners")]
async fn list_scanners(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<Value>, Status> {
    let rows = scanners::Entity::find()
        .filter(scanners::Column::OwnerId.eq(user.user_id))
        .order_by_desc(scanners::Column::LastSeenAt)
        .all(db.inner())
        .await
        .map_err(internal)?;
    Ok(Json(json!(
        rows.into_iter()
            .map(|s| {
                let mut v = s.payload;
                v["id"] = json!(s.id);
                v["lastSeenAt"] = json!(s.last_seen_at);
                v
            })
            .collect::<Vec<_>>()
    )))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReportOptions {
    event_ids: Vec<Uuid>,
    include_notes: bool,
    include_photos: bool,
    units: Preferences,
}
#[post("/brews/<brew_id>/reports", data = "<options>")]
async fn create_report(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    options: Json<ReportOptions>,
) -> Result<Json<Value>, Status> {
    publish_report(user, db, brew_id, options, None).await
}
#[put("/brews/<brew_id>/reports/<report_id>", data = "<options>")]
async fn refresh_report(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    report_id: &str,
    options: Json<ReportOptions>,
) -> Result<Json<Value>, Status> {
    publish_report(user, db, brew_id, options, Some(report_id)).await
}
async fn publish_report(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    options: Json<ReportOptions>,
    replace: Option<&str>,
) -> Result<Json<Value>, Status> {
    let id = uuid(brew_id)?;
    require_brew(db, user.user_id, id).await?;
    let previous = if let Some(report_id) = replace {
        Some(
            reports::Entity::find_by_id(uuid(report_id)?)
                .filter(reports::Column::OwnerId.eq(user.user_id))
                .filter(reports::Column::BrewId.eq(id))
                .one(db.inner())
                .await
                .map_err(internal)?
                .ok_or(Status::NotFound)?,
        )
    } else {
        None
    };
    if previous.is_none()
        && reports::Entity::find()
            .filter(reports::Column::BrewId.eq(id))
            .count(db.inner())
            .await
            .map_err(internal)?
            >= 20
    {
        return Err(Status::Forbidden);
    }
    if options.event_ids.len() > 1000 {
        return Err(Status::UnprocessableEntity);
    }
    let brew = brew_service::find_by_id(db, user.user_id, id)
        .await
        .map_err(internal)?
        .ok_or(Status::NotFound)?;
    let events = brew_event_service::find_by_brew(db, id, None, None)
        .await
        .map_err(internal)?;
    if options
        .event_ids
        .iter()
        .any(|id| !events.iter().any(|e| &e.id == id))
    {
        return Err(Status::UnprocessableEntity);
    }
    let token = previous
        .as_ref()
        .map(|r| r.token.clone())
        .unwrap_or_else(|| format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()));
    let selected=events.into_iter().filter(|e| options.event_ids.contains(&e.id)).map(|e| {
 let photos=if options.include_photos { e.attachments.iter().map(|a| json!({"id":a.id,"filename":a.filename,"url":format!("/api/v1/reports/{token}/photos/{}",a.id)})).collect::<Vec<_>>() } else { vec![] };
 json!({"label":e.label,"eventType":e.event_type,"eventTime":e.event_time,"gravityAtEvent":e.gravity_at_event,"tempAtEvent":e.temp_at_event,"notes":if options.include_notes { e.notes } else { None },"photos":photos})
 }).collect::<Vec<_>>();
    let rows = crate::services::reading_service::find_filtered(
        db,
        user.user_id,
        &shared::ReadingsQuery {
            brew_id: Some(id),
            hydrometer_id: None,
            since: None,
            until: None,
            limit: None,
        },
    )
    .await
    .map_err(internal)?;
    let payload = json!({"name":brew.name,"style":brew.style,"status":brew.status,"og":brew.og,"fg":brew.fg,"targetFg":brew.target_fg,"abv":brew.final_abv,"yeastStrain":brew.yeast_strain,"startDate":brew.start_date,"endDate":brew.end_date,"notes":if options.include_notes { brew.notes } else { None },"units":options.units,"events":selected,"readings":rows.into_iter().map(|r| json!({"recordedAt":r.recorded_at,"gravity":r.gravity,"temperatureF":r.temperature_f})).collect::<Vec<_>>(),"createdAt":chrono::Utc::now()});
    if let Some(mut previous) = previous.map(reports::ActiveModel::from) {
        let report_id = previous.id.clone().unwrap();
        previous.payload = Set(payload);
        previous.created_at = Set(chrono::Utc::now().into());
        previous.update(db.inner()).await.map_err(internal)?;
        return Ok(Json(json!({"id":report_id,"token":token})));
    }
    let report_id = Uuid::new_v4();
    reports::Entity::insert(reports::ActiveModel {
        id: Set(report_id),
        owner_id: Set(user.user_id),
        brew_id: Set(id),
        token: Set(token.clone()),
        payload: Set(payload),
        created_at: Set(chrono::Utc::now().into()),
    })
    .exec(db.inner())
    .await
    .map_err(internal)?;
    Ok(Json(json!({"id":report_id,"token":token})))
}
#[get("/brews/<brew_id>/reports")]
async fn list_reports(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
) -> Result<Json<Value>, Status> {
    let id = uuid(brew_id)?;
    require_brew(db, user.user_id, id).await?;
    let rows = reports::Entity::find()
        .filter(reports::Column::BrewId.eq(id))
        .filter(reports::Column::OwnerId.eq(user.user_id))
        .order_by_desc(reports::Column::CreatedAt)
        .all(db.inner())
        .await
        .map_err(internal)?;
    Ok(Json(json!(
        rows.iter()
            .map(|r| json!({"id":r.id,"token":r.token,"createdAt":r.created_at}))
            .collect::<Vec<_>>()
    )))
}
#[delete("/brews/<brew_id>/reports/<id>")]
async fn revoke(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    id: &str,
) -> Result<Status, Status> {
    let brew = uuid(brew_id)?;
    require_brew(db, user.user_id, brew).await?;
    let result = reports::Entity::delete_many()
        .filter(reports::Column::Id.eq(uuid(id)?))
        .filter(reports::Column::BrewId.eq(brew))
        .filter(reports::Column::OwnerId.eq(user.user_id))
        .exec(db.inner())
        .await
        .map_err(internal)?;
    if result.rows_affected == 0 {
        Err(Status::NotFound)
    } else {
        Ok(Status::NoContent)
    }
}
async fn public_report(db: &DatabaseConnection, token: &str) -> Result<reports::Model, Status> {
    if token.len() != 64 {
        return Err(Status::NotFound);
    }
    reports::Entity::find()
        .filter(reports::Column::Token.eq(token))
        .one(db)
        .await
        .map_err(internal)?
        .ok_or(Status::NotFound)
}
#[get("/reports/<token>")]
async fn report(
    db: &State<DatabaseConnection>,
    token: &str,
) -> Result<NoStore<Json<Value>>, Status> {
    Ok(no_store(Json(public_report(db, token).await?.payload)))
}
#[get("/reports/<token>/photos/<id>")]
async fn photo(
    db: &State<DatabaseConnection>,
    token: &str,
    id: &str,
) -> Result<NoStore<(ContentType, Vec<u8>)>, Status> {
    let r = public_report(db, token).await?;
    let id = uuid(id)?;
    let allowed = r.payload["events"].as_array().is_some_and(|events| {
        events.iter().any(|e| {
            e["photos"]
                .as_array()
                .is_some_and(|photos| photos.iter().any(|p| p["id"] == id.to_string()))
        })
    });
    if !allowed {
        return Err(Status::NotFound);
    }
    let a = event_attachments::Entity::find_by_id(id)
        .one(db.inner())
        .await
        .map_err(internal)?
        .ok_or(Status::NotFound)?;
    let content = match a.content_type.as_str() {
        "image/png" => ContentType::PNG,
        "image/jpeg" => ContentType::JPEG,
        "image/gif" => ContentType::GIF,
        "image/webp" => ContentType::new("image", "webp"),
        _ => return Err(Status::NotFound),
    };
    let bytes = tokio::fs::read(
        std::path::PathBuf::from(super::attachments::upload_dir()).join(a.storage_path),
    )
    .await
    .map_err(|_| Status::NotFound)?;
    Ok(no_store((content, bytes)))
}
pub fn routes() -> Vec<Route> {
    routes![
        get_preferences,
        set_preferences,
        heartbeat,
        list_scanners,
        create_report,
        refresh_report,
        list_reports,
        revoke,
        report,
        photo
    ]
}
