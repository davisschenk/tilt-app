//! Two-user isolation tests: user B must never be able to read, change or
//! delete user A's data, through any route, and both users must be able to own
//! a Tilt of the same color.
//!
//! Needs Postgres via `DATABASE_URL` (CI provides one). Each run migrates into
//! its own throwaway schema, so it never touches real tables.

use rocket::http::{ContentType, Cookie, Header, Method, Status};
use rocket::local::asynchronous::Client;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, Set,
};
use sea_orm_migration::MigratorTrait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::guards::current_user::SESSION_COOKIE;
use crate::models::entities::event_attachments;
use crate::routes;
use crate::services::{
    sessions::{self, CreateSessionParams},
    user_service,
};

struct Harness {
    client: Client,
    db: DatabaseConnection,
    admin: DatabaseConnection,
    schema: String,
    upload_dir: std::path::PathBuf,
    a: Cookie<'static>,
    b: Cookie<'static>,
}

impl Harness {
    async fn new() -> Self {
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must point at a Postgres the tests may create schemas in");
        let schema = format!("t_{}", Uuid::new_v4().simple());

        let admin = Database::connect(&url).await.expect("connect to Postgres");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .expect("create test schema");

        let mut opts = ConnectOptions::new(url);
        opts.set_schema_search_path(schema.clone())
            .max_connections(5);
        let db = Database::connect(opts)
            .await
            .expect("connect to test schema");
        migration::Migrator::up(&db, None)
            .await
            .expect("migrations apply to an empty schema");

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
            .mount("/api/v1", routes::account::routes());
        let client = Client::tracked(rocket).await.expect("rocket builds");

        Harness {
            client,
            db,
            admin,
            schema,
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
        let _ = self
            .admin
            .execute_unprepared(&format!("DROP SCHEMA {} CASCADE", self.schema))
            .await;
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

#[rocket::async_test]
async fn users_cannot_see_or_touch_each_others_data() {
    let h = Harness::new().await;
    scenario(&h).await;
    limits_scenario(&h).await;
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

    // An event is only reachable through the brew it belongs to.
    let (_, brew2) = h
        .post("/brews", a, json!({"name": "A brew 2", "hydrometerId": ha}))
        .await;
    let brew2 = id(&brew2);
    assert_eq!(
        h.get(&format!("/brews/{brew2}/events/{ev}"), a).await.0,
        Status::NotFound,
        "event is not reachable through a different brew"
    );

    // --- Attachments ---------------------------------------------------------------
    let att_id = Uuid::new_v4();
    let storage_path = format!("{ev}/{att_id}_pic.png");
    std::fs::create_dir_all(h.upload_dir.join(&ev)).unwrap();
    std::fs::write(h.upload_dir.join(&storage_path), b"not really a png").unwrap();
    event_attachments::ActiveModel {
        id: Set(att_id),
        event_id: Set(ev.parse().unwrap()),
        filename: Set("pic.png".into()),
        storage_path: Set(storage_path.clone()),
        content_type: Set("image/png".into()),
        size_bytes: Set(16),
        created_at: Set(chrono::Utc::now().fixed_offset()),
    }
    .insert(&h.db)
    .await
    .unwrap();
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
    let resp = h
        .client
        .post("/api/v1/readings")
        .header(Header::new("X-API-Key", raw_key.clone()))
        .json(&reading("Red"))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Created, "A's key ingests a reading");
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
    let resp = h
        .client
        .post("/api/v1/readings")
        .header(Header::new("X-API-Key", "0".repeat(64)))
        .json(&reading("Red"))
        .dispatch()
        .await;
    assert_eq!(resp.status(), Status::Unauthorized, "unknown API key");

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
        .manage(ApiUserLimiter::new(5))
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
        brew_service::create(&h.db, c_id, brew())
            .await
            .expect("under the cap");
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
