//! Two-user isolation tests: user B must never be able to read, change or
//! delete user A's data, through any route, and both users must be able to own
//! a Tilt of the same color.
//!
//! Needs Postgres via `DATABASE_URL` (CI provides one). Each run migrates into
//! its own throwaway schema, so it never touches real tables.

use rocket::http::{ContentType, Cookie, Header, Method, Status};
use rocket::local::asynchronous::Client;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Set,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::guards::current_user::SESSION_COOKIE;
use crate::models::entities::event_attachments;
use crate::routes;
use crate::services::{
    sessions::{self, CreateSessionParams},
    user_service,
};
use crate::test_db::TestDb;

struct Harness {
    client: Client,
    db: DatabaseConnection,
    test_db: TestDb,
    upload_dir: std::path::PathBuf,
    a: Cookie<'static>,
    b: Cookie<'static>,
}

impl Harness {
    async fn new() -> Self {
        let test_db = TestDb::new().await;
        let db = test_db.db.clone();
        let schema = test_db.schema.clone();

        let upload_dir = std::env::temp_dir().join(format!("tilt-test-{schema}"));
        std::fs::create_dir_all(&upload_dir).unwrap();
        // SAFETY: this module holds the only test that reads these variables, so
        // nothing else in the process races with the writes.
        unsafe {
            std::env::set_var("UPLOAD_DIR", &upload_dir);
        }

        let a = login(&db, "sub-user-a", "a@example.com").await;
        let b = login(&db, "sub-user-b", "b@example.com").await;

        let rocket = rocket::build()
            .manage(db.clone())
            .manage(crate::auth_mode::AuthMode::Oidc)
            .manage(reqwest::Client::new())
            .manage(crate::fairings::rate_limit::TestFireLimiter::new())
            .mount("/api/v1", routes::hydrometers::routes())
            .mount("/api/v1", routes::brews::routes())
            .mount("/api/v1", routes::readings::routes())
            .mount("/api/v1", routes::alert_targets::routes())
            .mount("/api/v1", routes::alert_rules::routes())
            .mount("/api/v1", routes::brew_events::routes())
            .mount("/api/v1", routes::attachments::routes())
            .mount("/api/v1", routes::analytics::routes())
            .mount("/api/v1", routes::tosna::routes())
            .mount("/api/v1", routes::api_keys::routes())
            .mount("/api/v1", routes::preferences::routes())
            .mount("/api/v1", routes::scanners::routes())
            .mount("/api/v1", routes::reports::routes())
            .mount("/api/v1", routes::account::routes());
        let client = Client::tracked(rocket).await.expect("rocket builds");

        Harness {
            client,
            db,
            test_db,
            upload_dir,
            a,
            b,
        }
    }

    async fn owner_of_opt(&self, sub: &str) -> Option<Uuid> {
        user_service::find_by_sub(&self.db, sub)
            .await
            .unwrap()
            .map(|u| u.id)
    }

    async fn owner_of(&self, sub: &str) -> Uuid {
        user_service::find_by_sub(&self.db, sub)
            .await
            .unwrap()
            .unwrap()
            .id
    }

    async fn cleanup(self) {
        self.test_db.cleanup().await;
        let _ = std::fs::remove_dir_all(&self.upload_dir);
    }

    /// Send a request as `who` (a session cookie), or anonymously.
    async fn call(
        &self,
        method: Method,
        uri: &str,
        who: Option<&Cookie<'static>>,
        body: Option<Value>,
    ) -> (Status, Value) {
        let mut req = self.client.req(method, format!("/api/v1{uri}"));
        if let Some(c) = who {
            req = req.private_cookie(c.clone());
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.dispatch().await;
        let status = resp.status();
        let json = resp.into_json::<Value>().await.unwrap_or(Value::Null);
        (status, json)
    }

    /// Send a request authenticated by an API key (as scanner firmware does),
    /// optionally naming the scanner device.
    async fn call_with_key(
        &self,
        method: Method,
        uri: &str,
        key: &str,
        scanner: Option<&str>,
        body: Value,
    ) -> Status {
        let mut req = self
            .client
            .req(method, format!("/api/v1{uri}"))
            .header(Header::new("X-API-Key", key.to_owned()))
            .json(&body);
        if let Some(device) = scanner {
            req = req.header(Header::new("X-Scanner-Id", device.to_owned()));
        }
        req.dispatch().await.status()
    }

    async fn get(&self, uri: &str, who: &Cookie<'static>) -> (Status, Value) {
        self.call(Method::Get, uri, Some(who), None).await
    }
    async fn post(&self, uri: &str, who: &Cookie<'static>, body: Value) -> (Status, Value) {
        self.call(Method::Post, uri, Some(who), Some(body)).await
    }
    async fn put(&self, uri: &str, who: &Cookie<'static>, body: Value) -> (Status, Value) {
        self.call(Method::Put, uri, Some(who), Some(body)).await
    }
    async fn delete(&self, uri: &str, who: &Cookie<'static>) -> Status {
        self.call(Method::Delete, uri, Some(who), None).await.0
    }
}

/// Create a user and a live session for them; return the session cookie.
async fn login(db: &DatabaseConnection, sub: &str, email: &str) -> Cookie<'static> {
    user_service::upsert(db, sub, email, sub).await.unwrap();
    let session = sessions::create_session(
        db,
        CreateSessionParams {
            user_sub: sub.to_string(),
            email: email.to_string(),
            name: sub.to_string(),
            id_token_hash: "test".to_string(),
            expires_in_secs: 3600,
        },
    )
    .await
    .unwrap();
    Cookie::new(SESSION_COOKIE, session.id.to_string())
}

fn id(v: &Value) -> String {
    v["id"].as_str().expect("response has an id").to_string()
}

fn reading(color: &str) -> Value {
    json!([{
        "color": color,
        "temperatureF": 68.0,
        "gravity": 1.050,
        "rssi": -60,
        "recordedAt": "2026-01-01T00:00:00Z"
    }])
}

/// A scanner heartbeat body for `device`.
fn heartbeat(device: &str) -> Value {
    json!({"deviceId": device, "firmwareVersion": "v1", "uptimeSeconds": 100,
           "wifiRssi": -50, "scanIntervalSeconds": 60, "bufferedReadings": 0,
           "scanOk": true, "uploadOk": true})
}

/// Put an attachment file on disk under `event_id` and its row in the database,
/// as an upload would. Returns the attachment id and its storage path.
async fn seed_attachment(
    h: &Harness,
    event_id: &str,
    filename: &str,
    bytes: &[u8],
) -> (Uuid, String) {
    let att_id = Uuid::new_v4();
    let storage_path = format!("{event_id}/{att_id}_{filename}");
    std::fs::create_dir_all(h.upload_dir.join(event_id)).unwrap();
    std::fs::write(h.upload_dir.join(&storage_path), bytes).unwrap();
    event_attachments::ActiveModel {
        id: Set(att_id),
        event_id: Set(event_id.parse().unwrap()),
        filename: Set(filename.into()),
        storage_path: Set(storage_path.clone()),
        content_type: Set("image/png".into()),
        size_bytes: Set(bytes.len() as i64),
        created_at: Set(chrono::Utc::now().fixed_offset()),
    }
    .insert(&h.db)
    .await
    .unwrap();
    (att_id, storage_path)
}

#[rocket::async_test]
async fn users_cannot_see_or_touch_each_others_data() {
    let h = Harness::new().await;
    brewer_features_scenario(&h).await;
    scenario(&h).await;
    limits_scenario(&h).await;
    account_deletion_scenario(&h).await;
    h.cleanup().await;
}

async fn scenario(h: &Harness) {
    let (a, b) = (&h.a, &h.b);

    // --- Unauthenticated requests are rejected ---------------------------------
    let (s, _) = h.call(Method::Get, "/hydrometers", None, None).await;
    assert_eq!(s, Status::Unauthorized, "anonymous list hydrometers");
    let (s, _) = h
        .call(Method::Post, "/readings", None, Some(reading("Red")))
        .await;
    assert_eq!(s, Status::Unauthorized, "anonymous post readings");

    // --- Hydrometers: both users can own a RED Tilt ----------------------------
    let (s, ha) = h
        .post("/hydrometers", a, json!({"color": "Red", "name": "A red"}))
        .await;
    assert_eq!(s, Status::Created, "A creates RED");
    let (s, hb) = h
        .post("/hydrometers", b, json!({"color": "Red", "name": "B red"}))
        .await;
    assert_eq!(
        s,
        Status::Created,
        "B creates RED too (color is unique per owner)"
    );
    let (ha, hb) = (id(&ha), id(&hb));

    let (_, list) = h.get("/hydrometers", b).await;
    assert_eq!(
        list.as_array().unwrap().len(),
        1,
        "B sees only their hydrometer"
    );
    assert_eq!(id(&list[0]), hb);

    assert_eq!(
        h.get(&format!("/hydrometers/{ha}"), b).await.0,
        Status::NotFound,
        "B GET A's hydrometer"
    );
    assert_eq!(
        h.put(&format!("/hydrometers/{ha}"), b, json!({"name": "pwned"}))
            .await
            .0,
        Status::NotFound,
        "B PUT A's hydrometer"
    );
    assert_eq!(
        h.delete(&format!("/hydrometers/{ha}"), b).await,
        Status::NotFound,
        "B DELETE A's hydrometer"
    );
    let (s, still) = h.get(&format!("/hydrometers/{ha}"), a).await;
    assert_eq!(s, Status::Ok, "A's hydrometer survives");
    assert_eq!(still["name"], "A red", "A's hydrometer is unchanged");

    // --- Brews ------------------------------------------------------------------
    let (s, brew) = h
        .post(
            "/brews",
            a,
            json!({"name": "A brew", "hydrometerId": ha, "og": 1.06, "targetFg": 1.01}),
        )
        .await;
    assert_eq!(s, Status::Created, "A creates brew");
    let brew = id(&brew);

    let (_, list) = h.get("/brews", b).await;
    assert!(list.as_array().unwrap().is_empty(), "B sees no brews");
    assert_eq!(
        h.get(&format!("/brews/{brew}"), b).await.0,
        Status::NotFound,
        "B GET A's brew"
    );
    assert_eq!(
        h.put(&format!("/brews/{brew}"), b, json!({"name": "x"}))
            .await
            .0,
        Status::NotFound,
        "B PUT A's brew"
    );
    assert_eq!(
        h.delete(&format!("/brews/{brew}"), b).await,
        Status::NotFound,
        "B DELETE A's brew"
    );
    assert_eq!(
        h.post("/brews", b, json!({"name": "steal", "hydrometerId": ha}))
            .await
            .0,
        Status::UnprocessableEntity,
        "B cannot attach a brew to A's hydrometer"
    );

    // --- Brew events, analytics, nutrient schedule -------------------------------
    let event_body = |brew: &str| json!({"brewId": brew, "eventType": "note", "label": "hi", "eventTime": "2026-01-01T00:00:00Z"});
    assert_eq!(
        h.post(&format!("/brews/{brew}/events"), b, event_body(&brew))
            .await
            .0,
        Status::NotFound,
        "B cannot add events to A's brew"
    );
    let (s, ev) = h
        .post(&format!("/brews/{brew}/events"), a, event_body(&brew))
        .await;
    assert_eq!(s, Status::Created, "A adds an event");
    let ev = id(&ev);
    assert_eq!(
        h.get(&format!("/brews/{brew}/events"), b).await.0,
        Status::NotFound,
        "B list A's events"
    );
    assert_eq!(
        h.get(&format!("/brews/{brew}/events/{ev}"), b).await.0,
        Status::NotFound,
        "B GET A's event"
    );
    assert_eq!(
        h.put(
            &format!("/brews/{brew}/events/{ev}"),
            b,
            json!({"label": "x"})
        )
        .await
        .0,
        Status::NotFound,
        "B PUT A's event"
    );
    assert_eq!(
        h.delete(&format!("/brews/{brew}/events/{ev}"), b).await,
        Status::NotFound,
        "B DELETE A's event"
    );
    assert_eq!(
        h.get(&format!("/brews/{brew}/analytics"), b).await.0,
        Status::NotFound,
        "B analytics on A's brew"
    );
    assert_eq!(
        h.get(&format!("/brews/{brew}/analytics"), a).await.0,
        Status::Ok,
        "A analytics"
    );
    assert_eq!(
        h.get(&format!("/brews/{brew}/nutrient-schedule"), b)
            .await
            .0,
        Status::NotFound,
        "B nutrient schedule"
    );

    // Keep one active brew per hydrometer while testing a second historical brew.
    assert_eq!(
        h.put(&format!("/brews/{brew}"), a, json!({"status":"Completed"}))
            .await
            .0,
        Status::Ok
    );
    // An event is only reachable through the brew it belongs to.
    let (_, brew2) = h
        .post("/brews", a, json!({"name": "A brew 2", "hydrometerId": ha}))
        .await;
    let brew2 = id(&brew2);
    assert_eq!(
        h.put(&format!("/brews/{brew2}"), a, json!({"status":"Completed"}))
            .await
            .0,
        Status::Ok
    );
    assert_eq!(
        h.put(&format!("/brews/{brew}"), a, json!({"status":"Active"}))
            .await
            .0,
        Status::Ok
    );
    assert_eq!(
        h.get(&format!("/brews/{brew2}/events/{ev}"), a).await.0,
        Status::NotFound,
        "event is not reachable through a different brew"
    );

    // --- Attachments ---------------------------------------------------------------
    let (att_id, storage_path) = seed_attachment(h, &ev, "pic.png", b"not really a png").await;
    assert_eq!(
        h.get(&format!("/attachments/{att_id}/file"), a).await.0,
        Status::Ok,
        "A fetches own attachment"
    );
    assert_eq!(
        h.get(&format!("/attachments/{att_id}/file"), b).await.0,
        Status::NotFound,
        "B fetches A's attachment"
    );
    assert_eq!(
        h.delete(
            &format!("/brews/{brew}/events/{ev}/attachments/{att_id}"),
            b
        )
        .await,
        Status::NotFound,
        "B deletes A's attachment"
    );
    assert!(
        h.upload_dir.join(&storage_path).exists(),
        "A's file is still on disk"
    );

    // --- Alert targets and rules ----------------------------------------------------
    let (s, target) = h
        .post(
            "/alert-targets",
            a,
            json!({"name": "t", "url": "https://example.com/hook", "format": "generic_json"}),
        )
        .await;
    assert_eq!(s, Status::Created, "A creates target");
    let target = id(&target);
    let (_, list) = h.get("/alert-targets", b).await;
    assert!(list.as_array().unwrap().is_empty(), "B sees no targets");
    assert_eq!(
        h.get(&format!("/alert-targets/{target}"), b).await.0,
        Status::NotFound,
        "B GET A's target"
    );
    assert_eq!(
        h.put(&format!("/alert-targets/{target}"), b, json!({"name": "x"}))
            .await
            .0,
        Status::NotFound,
        "B PUT A's target"
    );
    assert_eq!(
        h.delete(&format!("/alert-targets/{target}"), b).await,
        Status::NotFound,
        "B DELETE A's target"
    );
    let (_, fired) = h
        .post(&format!("/alert-targets/{target}/test"), b, json!(null))
        .await;
    assert_eq!(fired["ok"], false, "B cannot fire A's webhook");

    let (_, b_target) = h
        .post(
            "/alert-targets",
            b,
            json!({"name": "bt", "url": "https://example.com/b", "format": "generic_json"}),
        )
        .await;
    let b_target = id(&b_target);
    let rule = |target: &str, hydro: Option<&str>| {
        json!({"name": "r", "metric": "gravity", "operator": "lte", "threshold": 1.01,
               "alertTargetId": target, "hydrometerId": hydro, "enabled": false})
    };
    assert_eq!(
        h.post("/alert-rules", b, rule(&target, None)).await.0,
        Status::UnprocessableEntity,
        "B cannot point a rule at A's target"
    );
    assert_eq!(
        h.post("/alert-rules", b, rule(&b_target, Some(&ha)))
            .await
            .0,
        Status::UnprocessableEntity,
        "B cannot scope a rule to A's hydrometer"
    );
    assert_eq!(
        h.post(
            "/brews",
            b,
            json!({"name": "n", "hydrometerId": hb, "nutrientAlertTargetId": target})
        )
        .await
        .0,
        Status::UnprocessableEntity,
        "B cannot use A's target for nutrient alerts"
    );
    let (s, rule_a) = h.post("/alert-rules", a, rule(&target, Some(&ha))).await;
    assert_eq!(s, Status::Created, "A creates a rule");
    let rule_a = id(&rule_a);
    let (_, list) = h.get("/alert-rules", b).await;
    assert!(list.as_array().unwrap().is_empty(), "B sees no rules");
    assert_eq!(
        h.get(&format!("/alert-rules/{rule_a}"), b).await.0,
        Status::NotFound,
        "B GET A's rule"
    );
    assert_eq!(
        h.put(&format!("/alert-rules/{rule_a}"), b, json!({"name": "x"}))
            .await
            .0,
        Status::NotFound,
        "B PUT A's rule"
    );
    assert_eq!(
        h.delete(&format!("/alert-rules/{rule_a}"), b).await,
        Status::NotFound,
        "B DELETE A's rule"
    );

    // --- API keys and reading ingestion -----------------------------------------------
    let (s, key) = h.post("/api-keys", a, json!({"name": "pi"})).await;
    assert_eq!(s, Status::Created, "A creates an API key");
    let (raw_key, key_id) = (key["key"].as_str().unwrap().to_string(), id(&key));
    let (_, list) = h.get("/api-keys", b).await;
    assert!(list.as_array().unwrap().is_empty(), "B sees no API keys");
    assert_eq!(
        h.delete(&format!("/api-keys/{key_id}"), b).await,
        Status::NoContent,
        "B delete is a silent no-op"
    );
    let (_, list) = h.get("/api-keys", a).await;
    assert_eq!(
        list.as_array().unwrap().len(),
        1,
        "A's key survives B's delete"
    );

    // A's key writes into A's RED hydrometer, not B's.
    assert_eq!(
        h.call_with_key(Method::Post, "/readings", &raw_key, None, reading("Red"))
            .await,
        Status::Created,
        "A's key ingests a reading"
    );
    let (_, rs) = h.get(&format!("/readings?hydrometer_id={ha}"), a).await;
    assert_eq!(
        rs.as_array().unwrap().len(),
        1,
        "reading landed on A's hydrometer"
    );
    let (_, rs) = h.get(&format!("/readings?hydrometer_id={hb}"), b).await;
    assert!(
        rs.as_array().unwrap().is_empty(),
        "nothing landed on B's hydrometer"
    );

    // B can neither read A's readings nor guess around the filter.
    let (_, rs) = h.get(&format!("/readings?hydrometer_id={ha}"), b).await;
    assert!(
        rs.as_array().unwrap().is_empty(),
        "B reads A's readings by hydrometer id"
    );
    let (_, rs) = h.get(&format!("/readings?brew_id={brew}"), b).await;
    assert!(
        rs.as_array().unwrap().is_empty(),
        "B reads A's readings by brew id"
    );
    let (_, rs) = h.get("/readings", b).await;
    assert!(
        rs.as_array().unwrap().is_empty(),
        "B's unfiltered readings are empty"
    );

    // B posting via their session lands on B's hydrometer only.
    let (s, _) = h.post("/readings", b, reading("Red")).await;
    assert_eq!(s, Status::Created, "B ingests a reading via session");
    let (_, rs) = h.get(&format!("/readings?hydrometer_id={hb}"), b).await;
    assert_eq!(
        rs.as_array().unwrap().len(),
        1,
        "reading landed on B's hydrometer"
    );
    let (_, rs) = h.get(&format!("/readings?hydrometer_id={ha}"), a).await;
    assert_eq!(
        rs.as_array().unwrap().len(),
        1,
        "A's readings are unchanged"
    );

    // A garbage key is not a way in.
    assert_eq!(
        h.call_with_key(
            Method::Post,
            "/readings",
            &"0".repeat(64),
            None,
            reading("Red")
        )
        .await,
        Status::Unauthorized,
        "unknown API key"
    );

    // --- Upload hardening ---------------------------------------------------------
    let upload = |who: &Cookie<'static>,
                  brew: &str,
                  ev: &str,
                  name: &str,
                  claimed: &str,
                  bytes: &[u8]| {
        let mut body = format!(
            "--XBOUND\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: {claimed}\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n--XBOUND--\r\n");
        h.client
            .post(format!("/api/v1/brews/{brew}/events/{ev}/attachments"))
            .private_cookie(who.clone())
            .header(ContentType::new("multipart", "form-data").with_params(("boundary", "XBOUND")))
            .body(body)
            .dispatch()
    };

    let svg = b"<svg xmlns='http://www.w3.org/2000/svg' onload='alert(1)'/>";
    let resp = upload(a, &brew, &ev, "evil.png", "image/png", svg).await;
    assert_eq!(
        resp.status(),
        Status::UnprocessableEntity,
        "SVG labelled image/png is rejected"
    );
    let resp = upload(a, &brew, &ev, "evil.svg", "image/svg+xml", svg).await;
    assert_eq!(
        resp.status(),
        Status::UnprocessableEntity,
        "SVG labelled image/svg+xml is rejected"
    );

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&[0u8; 32]);
    let resp = upload(b, &brew, &ev, "x.png", "image/png", &png).await;
    assert_eq!(
        resp.status(),
        Status::NotFound,
        "B cannot upload to A's event"
    );

    let resp = upload(a, &brew, &ev, "photo.svg", "image/svg+xml", &png).await;
    assert_eq!(
        resp.status(),
        Status::Created,
        "real PNG is accepted whatever it is called"
    );
    let saved: Value = resp.into_json().await.unwrap();
    assert_eq!(
        saved["contentType"], "image/png",
        "stored type comes from the bytes"
    );
    assert_eq!(
        saved["filename"], "photo.png",
        "extension follows the detected type"
    );

    let resp = h
        .client
        .get(format!(
            "/api/v1/attachments/{}/file",
            saved["id"].as_str().unwrap()
        ))
        .private_cookie(a.clone())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.content_type(), Some(ContentType::PNG));
    assert_eq!(
        resp.headers().get_one("X-Content-Type-Options"),
        Some("nosniff")
    );
    let csp = resp
        .headers()
        .get_one("Content-Security-Policy")
        .unwrap_or_default();
    assert!(
        csp.contains("sandbox") && csp.contains("default-src 'none'"),
        "attachment CSP: {csp}"
    );

    // --- Webhook SSRF ---------------------------------------------------------------
    let target_body = |url: &str| json!({"name": "ssrf", "url": url, "format": "generic_json"});
    for url in [
        "http://example.com/hook",
        "https://127.0.0.1/hook",
        "https://169.254.169.254/latest/meta-data/",
        "https://localhost/hook",
        "https://db/hook",
        "https://[::1]/hook",
    ] {
        assert_eq!(
            h.post("/alert-targets", a, target_body(url)).await.0,
            Status::UnprocessableEntity,
            "create rejects {url}"
        );
        assert_eq!(
            h.put(
                &format!("/alert-targets/{target}"),
                a,
                json!({ "url": url })
            )
            .await
            .0,
            Status::UnprocessableEntity,
            "update rejects {url}"
        );
    }
    // A target saved before validation existed is still blocked when fired.
    let legacy = Uuid::new_v4();
    crate::models::entities::alert_targets::ActiveModel {
        id: Set(legacy),
        owner_id: Set(h.owner_of("sub-user-a").await),
        name: Set("legacy".into()),
        url: Set("https://169.254.169.254/latest/meta-data/".into()),
        format: Set("generic_json".into()),
        secret_header: Set(None),
        enabled: Set(true),
        created_at: Set(chrono::Utc::now().fixed_offset()),
        updated_at: Set(chrono::Utc::now().fixed_offset()),
    }
    .insert(&h.db)
    .await
    .unwrap();
    let (_, fired) = h
        .post(&format!("/alert-targets/{legacy}/test"), a, json!(null))
        .await;
    assert_eq!(fired["ok"], false, "legacy internal target must not fire");
    assert!(
        fired["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Blocked webhook URL"),
        "blocked, not merely failed to connect: {fired}"
    );

    // Test-fire is limited per user (10/min); we have used 3 for A already above.
    let mut limited = false;
    for _ in 0..12 {
        if h.post(&format!("/alert-targets/{legacy}/test"), a, json!(null))
            .await
            .0
            == Status::TooManyRequests
        {
            limited = true;
            break;
        }
    }
    assert!(limited, "test-fire is rate limited per user");
    let (s, _) = h
        .post(&format!("/alert-targets/{b_target}/test"), b, json!(null))
        .await;
    assert_eq!(s, Status::Ok, "B's limit is independent of A's");

    // --- Account export and deletion ----------------------------------------------
    // Give B a full footprint: brew, event, uploaded file (plus the reading, target
    // and API key from above).
    let (s, b_brew) = h
        .post("/brews", b, json!({"name": "B brew", "hydrometerId": hb}))
        .await;
    assert_eq!(s, Status::Created, "B creates a brew");
    let b_brew = id(&b_brew);
    let (_, b_ev) = h
        .post(&format!("/brews/{b_brew}/events"), b, event_body(&b_brew))
        .await;
    let b_ev = id(&b_ev);
    let resp = upload(b, &b_brew, &b_ev, "b.png", "image/png", &png).await;
    assert_eq!(resp.status(), Status::Created, "B uploads a photo");
    assert!(h.upload_dir.join(&b_ev).exists(), "B's upload is on disk");
    let (s, _) = h.post("/api-keys", b, json!({"name": "b-key"})).await;
    assert_eq!(s, Status::Created);

    // Export: A gets only A's data, and no secrets.
    let resp = h
        .client
        .get("/api/v1/account/export")
        .private_cookie(a.clone())
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Ok, "A exports");
    assert!(
        resp.headers()
            .get_one("Content-Disposition")
            .unwrap_or_default()
            .starts_with("attachment"),
        "export downloads as a file"
    );
    let export: Value = resp.into_json().await.unwrap();
    assert_eq!(export["account"]["email"], "a@example.com");
    assert_eq!(
        export["hydrometers"].as_array().unwrap().len(),
        1,
        "export has A's hydrometer"
    );
    assert_eq!(
        export["readings"].as_array().unwrap().len(),
        1,
        "export has A's reading"
    );
    let text = export.to_string();
    for leak in [
        hb.as_str(),
        b_brew.as_str(),
        "B brew",
        "b-key",
        "key_hash",
        "storage_path",
        raw_key.as_str(),
    ] {
        assert!(!text.contains(leak), "export must not contain {leak}");
    }
    assert!(
        export["alertTargets"][0].get("secret_header").is_none(),
        "webhook secrets are not exported"
    );
    assert_eq!(
        h.call(Method::Get, "/account/export", None, None).await.0,
        Status::Unauthorized,
        "anonymous export"
    );

    // Deletion needs the account email, then removes everything of B's and nothing of A's.
    let (s, _) = h
        .post(
            "/account/delete",
            b,
            json!({"confirmEmail": "wrong@example.com"}),
        )
        .await;
    assert_eq!(s, Status::UnprocessableEntity, "wrong confirmation email");
    assert!(
        h.get("/hydrometers", b).await.0 == Status::Ok,
        "B still exists after a refused delete"
    );
    let (s, _) = h
        .post(
            "/account/delete",
            b,
            json!({"confirmEmail": " B@Example.com "}),
        )
        .await;
    assert_eq!(s, Status::NoContent, "B deletes their account");

    assert_eq!(
        h.get("/hydrometers", b).await.0,
        Status::Unauthorized,
        "B's session is gone"
    );
    let b_id = h.owner_of_opt("sub-user-b").await;
    assert!(b_id.is_none(), "B's user row is gone");
    let db = &h.db;
    use crate::models::entities::{alert_targets, api_keys, brews, hydrometers, readings};
    assert_eq!(
        hydrometers::Entity::find()
            .filter(hydrometers::Column::Id.eq(hb.parse::<Uuid>().unwrap()))
            .count(db)
            .await
            .unwrap(),
        0,
        "B's hydrometer deleted"
    );
    assert_eq!(
        brews::Entity::find()
            .filter(brews::Column::Id.eq(b_brew.parse::<Uuid>().unwrap()))
            .count(db)
            .await
            .unwrap(),
        0,
        "B's brew deleted"
    );
    assert_eq!(
        alert_targets::Entity::find()
            .filter(alert_targets::Column::Id.eq(b_target.parse::<Uuid>().unwrap()))
            .count(db)
            .await
            .unwrap(),
        0,
        "B's target deleted"
    );
    assert_eq!(
        readings::Entity::find()
            .filter(readings::Column::HydrometerId.eq(hb.parse::<Uuid>().unwrap()))
            .count(db)
            .await
            .unwrap(),
        0,
        "B's readings deleted"
    );
    assert_eq!(
        api_keys::Entity::find().count(db).await.unwrap(),
        1,
        "only A's API key remains"
    );
    assert!(
        !h.upload_dir.join(&b_ev).exists(),
        "B's uploaded files are removed from disk"
    );

    // A is untouched.
    let (_, list) = h.get("/hydrometers", a).await;
    assert_eq!(
        list.as_array().unwrap().len(),
        1,
        "A's hydrometer survives B's deletion"
    );
    let (_, rs) = h.get(&format!("/readings?hydrometer_id={ha}"), a).await;
    assert_eq!(rs.as_array().unwrap().len(), 1, "A's reading survives");
    assert!(h.upload_dir.join(&ev).exists(), "A's files survive");
}

/// Per-user request limiting and per-account resource caps, on a fresh user so
/// the counts above are not disturbed.
async fn limits_scenario(h: &Harness) {
    use crate::fairings::rate_limit::ApiUserLimiter;
    use crate::services::{WriteError, alert_target_service, api_keys, brew_service, limits};
    use shared::{CreateAlertTarget, CreateBrew, WebhookFormat};

    let c = login(&h.db, "sub-user-c", "c@example.com").await;
    let c_id = h.owner_of("sub-user-c").await;

    // --- Per-user request limit: follows the account, not the address ---------------
    let limited = rocket::build()
        .manage(h.db.clone())
        .manage(crate::auth_mode::AuthMode::Oidc)
        .manage(ApiUserLimiter::new(std::num::NonZeroU32::new(5).unwrap()))
        .mount("/api/v1", routes::hydrometers::routes());
    let limited = Client::tracked(limited).await.unwrap();
    let hit = |who: &Cookie<'static>| {
        limited
            .get("/api/v1/hydrometers")
            .private_cookie(who.clone())
            .dispatch()
    };
    for i in 0..5 {
        assert_eq!(
            hit(&c).await.status(),
            Status::Ok,
            "request {i} is within the limit"
        );
    }
    assert_eq!(
        hit(&c).await.status(),
        Status::TooManyRequests,
        "sixth request in a minute"
    );
    assert_eq!(
        hit(&h.a).await.status(),
        Status::Ok,
        "another user is not affected"
    );

    // --- Resource caps ----------------------------------------------------------------
    let (_, hc) = h.post("/hydrometers", &c, json!({"color": "Red"})).await;
    let hc = id(&hc);
    let hc_uuid: Uuid = hc.parse().unwrap();

    let target = |n: u64| CreateAlertTarget {
        name: format!("t{n}"),
        url: "https://example.com/hook".into(),
        format: WebhookFormat::GenericJson,
        secret_header: None,
        enabled: None,
    };
    for n in 0..limits::MAX_ALERT_TARGETS {
        alert_target_service::create(&h.db, c_id, target(n))
            .await
            .expect("under the cap");
    }
    assert!(
        matches!(
            alert_target_service::create(&h.db, c_id, target(99)).await,
            Err(WriteError::LimitReached(_))
        ),
        "alert target cap"
    );
    assert_eq!(
        h.post(
            "/alert-targets",
            &c,
            json!({"name": "x", "url": "https://example.com/h", "format": "generic_json"})
        )
        .await
        .0,
        Status::Forbidden,
        "cap surfaces as 403 over HTTP"
    );
    let (s, _) = h
        .post(
            "/alert-targets",
            &h.b,
            json!({"name": "x", "url": "https://example.com/h", "format": "generic_json"}),
        )
        .await;
    assert_ne!(s, Status::Forbidden, "another user's cap is independent");

    for n in 0..limits::MAX_API_KEYS {
        api_keys::create_api_key(&h.db, c_id, format!("k{n}"), "sub-user-c".into(), None)
            .await
            .expect("under the cap");
    }
    assert!(
        matches!(
            api_keys::create_api_key(
                &h.db,
                c_id,
                "one too many".into(),
                "sub-user-c".into(),
                None
            )
            .await,
            Err(api_keys::ApiKeyError::LimitReached)
        ),
        "API key cap"
    );
    assert_eq!(
        h.post("/api-keys", &c, json!({"name": "x"})).await.0,
        Status::Forbidden,
        "key cap over HTTP"
    );

    let brew = || CreateBrew {
        name: "b".into(),
        hydrometer_id: hc_uuid,
        style: None,
        og: None,
        target_fg: None,
        notes: None,
        batch_size_gallons: None,
        yeast_nitrogen_requirement: None,
        pitch_time: None,
        nutrient_protocol: None,
        yeast_strain: None,
        nutrient_alert_target_id: None,
    };
    for _ in 0..limits::MAX_BREWS {
        let created = brew_service::create(&h.db, c_id, brew())
            .await
            .expect("under the cap");
        crate::models::entities::brews::Entity::update_many()
            .col_expr(
                crate::models::entities::brews::Column::Status,
                sea_orm::sea_query::Expr::value("Completed"),
            )
            .filter(crate::models::entities::brews::Column::Id.eq(created.id))
            .exec(&h.db)
            .await
            .unwrap();
    }
    assert!(
        matches!(
            brew_service::create(&h.db, c_id, brew()).await,
            Err(WriteError::LimitReached(_))
        ),
        "brew cap"
    );
    assert_eq!(
        h.post("/brews", &c, json!({"name": "x", "hydrometerId": hc}))
            .await
            .0,
        Status::Forbidden,
        "brew cap over HTTP"
    );

    // --- Reading ingest caps ------------------------------------------------------------
    let big: Vec<Value> = (0..=limits::MAX_READINGS_PER_BATCH)
        .map(|_| reading("Red")[0].clone())
        .collect();
    assert_eq!(
        h.post("/readings", &c, json!(big)).await.0,
        Status::PayloadTooLarge,
        "oversized batch"
    );

    assert_eq!(
        h.post("/readings", &c, reading("Red")).await.0,
        Status::Created,
        "under the daily cap"
    );
    h.db.execute_unprepared(&format!(
        "INSERT INTO readings (id, hydrometer_id, temperature_f, gravity, recorded_at, created_at) \
         SELECT gen_random_uuid(), '{hc}', 68, 1.05, now(), now() FROM generate_series(1, {})",
        limits::MAX_READINGS_PER_DAY
    ))
    .await
    .unwrap();
    assert_eq!(
        h.post("/readings", &c, reading("Red")).await.0,
        Status::TooManyRequests,
        "daily ingest cap"
    );
    // Backdated timestamps do not dodge it: the cap counts arrival time.
    let mut old = reading("Red");
    old[0]["recordedAt"] = json!("2020-01-01T00:00:00Z");
    assert_eq!(
        h.post("/readings", &c, old).await.0,
        Status::TooManyRequests,
        "backdated reading"
    );
    // 100,000 readings with the same timestamp must not hang the chart query: the
    // bucket width used to round to zero and the downsampler looped forever.
    let same_ts = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        h.get(&format!("/readings?hydrometer_id={hc}"), &c),
    )
    .await
    .expect("readings query with identical timestamps must finish");
    assert_eq!(same_ts.0, Status::Ok);
    assert!(
        !same_ts.1.as_array().unwrap().is_empty() && same_ts.1.as_array().unwrap().len() <= 5_001,
        "identical timestamps collapse to a few points"
    );

    // Long brews are downsampled while streaming: ~12,000 evenly spaced readings
    // come back as about 5,000, in time order, covering the whole range.
    let d = login(&h.db, "sub-user-d", "d@example.com").await;
    let (_, hd) = h.post("/hydrometers", &d, json!({"color": "Purple"})).await;
    let hd = id(&hd);
    h.db.execute_unprepared(&format!(
        "INSERT INTO readings (id, hydrometer_id, temperature_f, gravity, recorded_at, created_at) \
         SELECT gen_random_uuid(), '{hd}', 68, 1.05 - i * 0.000001, \
                timestamptz '2026-01-01 00:00:00+00' + i * interval '30 seconds', now() \
         FROM generate_series(0, 11999) AS i"
    ))
    .await
    .unwrap();
    let (s, rs) = h.get(&format!("/readings?hydrometer_id={hd}"), &d).await;
    assert_eq!(s, Status::Ok);
    let rs = rs.as_array().unwrap();
    assert!(
        (4_500..=5_001).contains(&rs.len()),
        "downsampled to ~5000, got {}",
        rs.len()
    );
    let times: Vec<&str> = rs
        .iter()
        .map(|r| r["recordedAt"].as_str().unwrap())
        .collect();
    assert!(
        times.windows(2).all(|w| w[0] < w[1]),
        "points are in strictly increasing time order"
    );
    assert!(
        times[0] < "2026-01-01T00:02",
        "starts at the beginning of the range"
    );
    assert!(
        *times.last().unwrap() > "2026-01-05T00:00",
        "reaches the end of the range (~4.2 days)"
    );
    // A caller-supplied limit still bounds the rows exactly.
    let (_, rs) = h
        .get(&format!("/readings?hydrometer_id={hd}&limit=10"), &d)
        .await;
    assert_eq!(rs.as_array().unwrap().len(), 10, "limit is honoured");

    // Other accounts still ingest.
    assert_eq!(
        h.post("/readings", &h.a, reading("Red")).await.0,
        Status::Created,
        "A is unaffected"
    );
}

async fn brewer_features_scenario(h: &Harness) {
    let a = login(&h.db, "feature-a", "feature-a@example.com").await;
    let b = login(&h.db, "feature-b", "feature-b@example.com").await;
    assert_eq!(
        h.get("/account/preferences", &a).await.1,
        json!({"temperature":"F","gravity":"SG"})
    );
    assert_eq!(
        h.put(
            "/account/preferences",
            &a,
            json!({"temperature":"C","gravity":"P"})
        )
        .await
        .0,
        Status::Ok
    );
    assert_eq!(
        h.get("/account/preferences", &a).await.1["temperature"],
        "C"
    );
    assert_eq!(
        h.get("/account/preferences", &b).await.1["temperature"],
        "F"
    );
    assert_eq!(
        h.put(
            "/account/preferences",
            &a,
            json!({"temperature":"K","gravity":"P"})
        )
        .await
        .0,
        Status::UnprocessableEntity
    );
    assert_eq!(
        h.post("/scanners/heartbeat", &a, heartbeat("scanner-a"))
            .await
            .0,
        Status::NoContent
    );
    assert_eq!(
        h.post("/scanners/heartbeat", &a, heartbeat("scanner-a"))
            .await
            .0,
        Status::NoContent
    );
    let scanners = h.get("/scanners", &a).await.1;
    assert_eq!(
        scanners.as_array().unwrap().len(),
        1,
        "heartbeat upserts stable device"
    );
    assert_eq!(h.get("/scanners", &b).await.1, json!([]));
    let (_, hydro) = h.post("/hydrometers", &a, json!({"color":"Blue"})).await;
    let hd = id(&hydro);
    let (status,brew)=h.post("/brews",&a,json!({"name":"Report batch","hydrometerId":hd,"og":1.05,"targetFg":1.01,"notes":"private recipe","yeastStrain":"US-05","batchSizeGallons":5})).await;
    assert_eq!(status, Status::Created);
    let brew_id = id(&brew);
    assert_eq!(
        h.post(
            "/brews",
            &a,
            json!({"name":"Conflicting duplicate","hydrometerId":hd})
        )
        .await
        .0,
        Status::UnprocessableEntity
    );

    let (_, key) = h.post("/api-keys", &a, json!({"name":"scanner key"})).await;
    let raw = key["key"].as_str().unwrap();
    assert_eq!(
        h.call_with_key(
            Method::Post,
            "/scanners/heartbeat",
            raw,
            None,
            heartbeat("scanner-a")
        )
        .await,
        Status::NoContent,
        "firmware authenticates with existing API key"
    );
    assert_eq!(
        h.call_with_key(
            Method::Post,
            "/readings",
            raw,
            Some("scanner-a"),
            reading("Blue")
        )
        .await,
        Status::Created
    );
    let row = crate::models::entities::readings::Entity::find()
        .filter(
            crate::models::entities::readings::Column::HydrometerId.eq(hd.parse::<Uuid>().unwrap()),
        )
        .one(&h.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.scanner_id,
        Some(scanners[0]["id"].as_str().unwrap().parse().unwrap())
    );
    let (_,event)=h.post(&format!("/brews/{brew_id}/events"),&a,json!({"brewId":brew_id,"eventType":"dry_hop","label":"Dry hop","notes":"private entry","eventTime":"2026-01-01T00:00:00Z"})).await;
    let event_id = id(&event);
    let options = json!({"eventIds":[event_id],"includeNotes":false,"includePhotos":false,"units":{"temperature":"C","gravity":"P"}});
    assert_eq!(
        h.post(&format!("/brews/{brew_id}/reports"), &b, options.clone())
            .await
            .0,
        Status::NotFound
    );
    let (status, r) = h
        .post(&format!("/brews/{brew_id}/reports"), &a, options.clone())
        .await;
    assert_eq!(status, Status::Ok);
    let token = r["token"].as_str().unwrap();
    let report_id = id(&r);
    let (status, snapshot) = h
        .call(Method::Get, &format!("/reports/{token}"), None, None)
        .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(snapshot["name"], "Report batch");
    assert!(snapshot["notes"].is_null());
    assert!(snapshot["events"][0]["notes"].is_null());
    assert!(snapshot.get("hydrometerId").is_none());
    assert_eq!(snapshot["units"]["temperature"], "C");
    h.put(&format!("/brews/{brew_id}"), &a, json!({"name":"Renamed"}))
        .await;
    assert_eq!(
        h.call(Method::Get, &format!("/reports/{token}"), None, None)
            .await
            .1["name"],
        "Report batch",
        "published snapshot stays fixed"
    );
    assert_eq!(
        h.put(
            &format!("/brews/{brew_id}/reports/{report_id}"),
            &b,
            options.clone()
        )
        .await
        .0,
        Status::NotFound
    );
    let refreshed = h
        .put(
            &format!("/brews/{brew_id}/reports/{report_id}"),
            &a,
            options.clone(),
        )
        .await;
    assert_eq!(refreshed.0, Status::Ok);
    assert_eq!(refreshed.1["token"], token, "refresh keeps existing link");
    assert_eq!(
        h.call(Method::Get, &format!("/reports/{token}"), None, None)
            .await
            .1["name"],
        "Renamed"
    );

    let (photo_id, _) = seed_attachment(h, &event_id, "photo.png", b"\x89PNG\r\n\x1a\nphoto").await;
    let photo_uri = format!("/reports/{token}/photos/{photo_id}");
    assert_eq!(
        h.call(Method::Get, &photo_uri, None, None).await.0,
        Status::NotFound,
        "unselected photo is private"
    );
    let mut with_photos = options.clone();
    with_photos["includePhotos"] = json!(true);
    with_photos["includeNotes"] = json!(true);
    assert_eq!(
        h.put(
            &format!("/brews/{brew_id}/reports/{report_id}"),
            &a,
            with_photos
        )
        .await
        .0,
        Status::Ok
    );
    let response = h.client.get(format!("/api/v1{photo_uri}")).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-store")
    );
    assert_eq!(
        h.delete(&format!("/brews/{brew_id}/reports/{report_id}"), &b)
            .await,
        Status::NotFound
    );
    assert_eq!(
        h.delete(&format!("/brews/{brew_id}/reports/{report_id}"), &a)
            .await,
        Status::NoContent
    );
    assert_eq!(
        h.call(Method::Get, &format!("/reports/{token}"), None, None)
            .await
            .0,
        Status::NotFound
    );
    assert_eq!(
        h.call(Method::Get, &photo_uri, None, None).await.0,
        Status::NotFound,
        "revocation also removes public photo access"
    );
    let (_, r) = h
        .post(&format!("/brews/{brew_id}/reports"), &a, options)
        .await;
    assert_eq!(
        h.delete(&format!("/brews/{brew_id}"), &a).await,
        Status::NoContent
    );
    assert_eq!(
        h.call(
            Method::Get,
            &format!("/reports/{}", r["token"].as_str().unwrap()),
            None,
            None
        )
        .await
        .0,
        Status::NotFound
    );
    assert_eq!(
        h.post(
            "/account/delete",
            &a,
            json!({"confirmEmail":"feature-a@example.com"})
        )
        .await
        .0,
        Status::NoContent
    );
    assert_eq!(
        h.post(
            "/account/delete",
            &b,
            json!({"confirmEmail":"feature-b@example.com"})
        )
        .await
        .0,
        Status::NoContent
    );
}

/// Everything one user put in: ids needed to find their rows again afterwards.
struct Footprint {
    user_id: Uuid,
    sub: String,
    cookie: Cookie<'static>,
    api_key: String,
    event_dir: String,
    report_token: String,
    photo_id: Uuid,
}

/// Create a row in every user-owned table (and an attachment file on disk) for a
/// fresh user, through the API wherever there is a route for it.
async fn full_footprint(h: &Harness, sub: &str, email: &str, color: &str) -> Footprint {
    let c = login(&h.db, sub, email).await;
    let user_id = h.owner_of(sub).await;

    // brewer_preferences, scanners
    let (s, _) = h
        .put(
            "/account/preferences",
            &c,
            json!({"temperature": "C", "gravity": "P"}),
        )
        .await;
    assert_eq!(s, Status::Ok, "{sub} sets preferences");
    let (s, _) = h
        .post("/scanners/heartbeat", &c, heartbeat("scanner-1"))
        .await;
    assert_eq!(s, Status::NoContent, "{sub} scanner heartbeat");
    // hydrometers, brews
    let (s, hydro) = h.post("/hydrometers", &c, json!({"color": color})).await;
    assert_eq!(s, Status::Created, "{sub} creates hydrometer");
    let hydro = id(&hydro);
    let (s, brew) = h
        .post(
            "/brews",
            &c,
            json!({"name": format!("{sub} brew"), "hydrometerId": hydro}),
        )
        .await;
    assert_eq!(s, Status::Created, "{sub} creates brew");
    let brew = id(&brew);
    // api_keys, and a reading tagged with the scanner (readings.scanner_id)
    let (s, key) = h.post("/api-keys", &c, json!({"name": "k"})).await;
    assert_eq!(s, Status::Created, "{sub} creates API key");
    let api_key = key["key"].as_str().unwrap().to_string();
    assert_eq!(
        h.call_with_key(
            Method::Post,
            "/readings",
            &api_key,
            Some("scanner-1"),
            reading(color)
        )
        .await,
        Status::Created,
        "{sub} posts a reading"
    );
    // brew_events, event_attachments (+ file on disk)
    let (s, ev) = h
        .post(
            &format!("/brews/{brew}/events"),
            &c,
            json!({"brewId": brew, "eventType": "note", "label": "L", "notes": "n",
                   "eventTime": "2026-01-01T00:00:00Z"}),
        )
        .await;
    assert_eq!(s, Status::Created, "{sub} creates event");
    let ev = id(&ev);
    let (photo_id, _) = seed_attachment(h, &ev, "photo.png", b"\x89PNG\r\n\x1a\nphoto").await;
    // alert_targets, alert_rules
    let (s, target) = h
        .post(
            "/alert-targets",
            &c,
            json!({"name": "t", "url": "https://example.com/hook", "format": "generic_json"}),
        )
        .await;
    assert_eq!(s, Status::Created, "{sub} creates target");
    let target = id(&target);
    let (s, _) = h
        .post(
            "/alert-rules",
            &c,
            json!({"name": "r", "metric": "gravity", "operator": "lte", "threshold": 1.01,
                   "alertTargetId": target, "brewId": brew, "enabled": false}),
        )
        .await;
    assert_eq!(s, Status::Created, "{sub} creates rule");
    // brew_reports, published with the photo
    let (s, report) = h
        .post(
            &format!("/brews/{brew}/reports"),
            &c,
            json!({"eventIds": [ev], "includeNotes": true, "includePhotos": true,
                   "units": {"temperature": "C", "gravity": "P"}}),
        )
        .await;
    assert_eq!(s, Status::Ok, "{sub} publishes a report");
    let report_token = report["token"].as_str().unwrap().to_string();

    Footprint {
        user_id,
        sub: sub.to_string(),
        cookie: c,
        api_key,
        event_dir: ev,
        report_token,
        photo_id,
    }
}

/// Tables that hold no user data, so account deletion and export ignore them.
/// Every other table in the schema must be covered by `owned_rows`.
const NON_USER_TABLES: [&str; 1] = [
    "seaql_migrations", // migration bookkeeping
];

/// Tables with an `owner_id` column pointing at the user.
const OWNER_ID_TABLES: [&str; 7] = [
    "brewer_preferences",
    "scanners",
    "hydrometers",
    "brews",
    "brew_reports",
    "api_keys",
    "alert_targets",
];

/// User-owned tables and the account export key each one appears under.
const EXPORT_KEYS: [(&str, &str); 12] = [
    ("users", "account"),
    ("brewer_preferences", "preferences"),
    ("scanners", "scanners"),
    ("hydrometers", "hydrometers"),
    ("brews", "brews"),
    ("brew_reports", "brewReports"),
    ("api_keys", "apiKeys"),
    ("alert_targets", "alertTargets"),
    ("alert_rules", "alertRules"),
    ("readings", "readings"),
    ("brew_events", "brewEvents"),
    ("event_attachments", "attachments"),
];

/// User-owned tables intentionally left out of the export.
const NOT_EXPORTED: [&str; 1] = [
    "user_sessions", // login session credentials, not user content
];

/// Row counts of everything belonging to the user, keyed by table. A table can
/// appear more than once when it is reachable by more than one path.
async fn owned_rows(db: &DatabaseConnection, f: &Footprint) -> Vec<(&'static str, u64)> {
    let by_id = || sea_orm::Value::from(f.user_id);
    let mut out = Vec::new();
    for table in OWNER_ID_TABLES {
        let sql = format!("SELECT count(*) FROM {table} WHERE owner_id = $1");
        out.push((table, count(db, &sql, by_id()).await));
    }
    for (table, sql, value) in [
        ("users", "SELECT count(*) FROM users WHERE id = $1", by_id()),
        (
            "user_sessions",
            "SELECT count(*) FROM user_sessions WHERE user_sub = $1",
            f.sub.clone().into(),
        ),
        (
            "alert_rules",
            "SELECT count(*) FROM alert_rules r JOIN alert_targets t ON t.id = r.alert_target_id \
             WHERE t.owner_id = $1",
            by_id(),
        ),
        (
            "readings",
            "SELECT count(*) FROM readings r JOIN hydrometers h ON h.id = r.hydrometer_id \
             WHERE h.owner_id = $1",
            by_id(),
        ),
        (
            "readings",
            "SELECT count(*) FROM readings r JOIN scanners s ON s.id = r.scanner_id \
             WHERE s.owner_id = $1",
            by_id(),
        ),
        (
            "brew_events",
            "SELECT count(*) FROM brew_events e JOIN brews b ON b.id = e.brew_id \
             WHERE b.owner_id = $1",
            by_id(),
        ),
        (
            "event_attachments",
            "SELECT count(*) FROM event_attachments a JOIN brew_events e ON e.id = a.event_id \
             JOIN brews b ON b.id = e.brew_id WHERE b.owner_id = $1",
            by_id(),
        ),
    ] {
        out.push((table, count(db, sql, value).await));
    }
    out
}

async fn count(db: &DatabaseConnection, sql: &str, value: sea_orm::Value) -> u64 {
    let row = db
        .query_one(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            [value],
        ))
        .await
        .unwrap()
        .unwrap();
    row.try_get_by_index::<i64>(0).unwrap() as u64
}

/// Every base table in the test schema, minus `NON_USER_TABLES`.
async fn user_tables(db: &DatabaseConnection) -> std::collections::BTreeSet<String> {
    db.query_all(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT table_name::text FROM information_schema.tables \
         WHERE table_schema = current_schema() AND table_type = 'BASE TABLE'",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.try_get_by_index::<String>(0).unwrap())
    .filter(|t| !NON_USER_TABLES.contains(&t.as_str()))
    .collect()
}

/// Check the public report link and the photo it publishes.
async fn assert_links(h: &Harness, f: &Footprint, want: Status, who: &str) {
    let report = format!("/reports/{}", f.report_token);
    let photo = format!("{report}/photos/{}", f.photo_id);
    assert_eq!(
        h.call(Method::Get, &report, None, None).await.0,
        want,
        "{who}'s published report link"
    );
    assert_eq!(
        h.call(Method::Get, &photo, None, None).await.0,
        want,
        "{who}'s published photo link"
    );
}

/// Account export includes, and account deletion removes, every user-owned table
/// (including scanners, brew reports and preferences), the uploaded files, the
/// sessions and API keys, and the published report links; another user's data
/// is untouched. A migration that adds a table fails this test until the table
/// is covered here (and by `account_service`).
async fn account_deletion_scenario(h: &Harness) {
    use std::collections::BTreeSet;

    let x = full_footprint(h, "del-x", "del-x@example.com", "Green").await;
    let y = full_footprint(h, "del-y", "del-y@example.com", "Green").await;

    // Structural completeness: every table is counted, and exported or excluded on purpose.
    let x_before = owned_rows(&h.db, &x).await;
    let counted: BTreeSet<String> = x_before.iter().map(|(t, _)| t.to_string()).collect();
    let schema = user_tables(&h.db).await;
    assert_eq!(
        schema, counted,
        "every table must be covered by owned_rows (or listed in NON_USER_TABLES)"
    );
    let exported: BTreeSet<String> = EXPORT_KEYS
        .iter()
        .map(|(t, _)| *t)
        .chain(NOT_EXPORTED)
        .map(str::to_string)
        .collect();
    assert_eq!(
        schema, exported,
        "every table must map to an export key (or be listed in NOT_EXPORTED)"
    );
    for (table, n) in &x_before {
        assert!(*n > 0, "test setup: X has a row in {table}");
    }
    let y_before = owned_rows(&h.db, &y).await;

    // Export covers every user-content table.
    let (s, export) = h.get("/account/export", &x.cookie).await;
    assert_eq!(s, Status::Ok, "X exports");
    for (table, key) in EXPORT_KEYS {
        let v = &export[key];
        let empty = v.is_null()
            || v.as_array().is_some_and(Vec::is_empty)
            || v.as_object().is_some_and(serde_json::Map::is_empty);
        assert!(!empty, "export key {key} has X's {table}");
    }
    assert_eq!(export["preferences"]["temperature"], "C");
    assert_eq!(
        export["readings"][0]["scanner_id"], export["scanners"][0]["id"],
        "exported reading links to the exported scanner"
    );
    let report = &export["brewReports"][0];
    assert_eq!(report["token"], x.report_token.as_str());
    assert!(
        report["payload"].get("readings").is_none(),
        "report snapshots do not duplicate readings"
    );
    assert!(
        !export.to_string().contains(&y.report_token),
        "export has none of Y's reports"
    );

    assert_links(h, &x, Status::Ok, "X").await;
    let (s, _) = h
        .post(
            "/account/delete",
            &x.cookie,
            json!({"confirmEmail": "del-x@example.com"}),
        )
        .await;
    assert_eq!(s, Status::NoContent, "X deletes their account");

    // Nothing of X's remains: rows, files, credentials, public links.
    for (table, n) in owned_rows(&h.db, &x).await {
        assert_eq!(n, 0, "X's {table} rows are gone");
    }
    assert!(
        !h.upload_dir.join(&x.event_dir).exists(),
        "X's uploaded files are removed from disk"
    );
    assert_eq!(
        h.get("/scanners", &x.cookie).await.0,
        Status::Unauthorized,
        "X's session no longer works"
    );
    assert_eq!(
        h.call_with_key(
            Method::Post,
            "/scanners/heartbeat",
            &x.api_key,
            None,
            heartbeat("scanner-1")
        )
        .await,
        Status::Unauthorized,
        "X's API key no longer works"
    );
    assert_links(h, &x, Status::NotFound, "X").await;

    // Y is untouched.
    assert_eq!(
        owned_rows(&h.db, &y).await,
        y_before,
        "Y's rows survive X's deletion"
    );
    assert!(
        h.upload_dir.join(&y.event_dir).exists(),
        "Y's files survive"
    );
    assert_links(h, &y, Status::Ok, "Y").await;
    assert_eq!(h.get("/scanners", &y.cookie).await.0, Status::Ok);
}
