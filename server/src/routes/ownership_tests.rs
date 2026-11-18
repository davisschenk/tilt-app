//! Two-user isolation tests: user B must never be able to read, change or
//! delete user A's data, through any route, and both users must be able to own
//! a Tilt of the same color.
//!
//! Needs Postgres via `DATABASE_URL` (CI provides one). Each run migrates into
//! its own throwaway schema, so it never touches real tables.

use rocket::http::{ContentType, Cookie, Header, Method, Status};
use rocket::local::asynchronous::Client;
use sea_orm::{
    ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Set,
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
            std::env::set_var("AUTH_MODE", "oidc");
            std::env::set_var("UPLOAD_DIR", &upload_dir);
        }

        let a = login(&db, "sub-user-a", "a@example.com").await;
        let b = login(&db, "sub-user-b", "b@example.com").await;

        let rocket = rocket::build()
            .manage(db.clone())
            .manage(reqwest::Client::new())
            .mount("/api/v1", routes::hydrometers::routes())
            .mount("/api/v1", routes::brews::routes())
            .mount("/api/v1", routes::readings::routes())
            .mount("/api/v1", routes::alert_targets::routes())
            .mount("/api/v1", routes::alert_rules::routes())
            .mount("/api/v1", routes::brew_events::routes())
            .mount("/api/v1", routes::attachments::routes())
            .mount("/api/v1", routes::analytics::routes())
            .mount("/api/v1", routes::tosna::routes())
            .mount("/api/v1", routes::api_keys::routes());
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
}
