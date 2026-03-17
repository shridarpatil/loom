//! Integration tests for additional HTTP API endpoints.

use loom_tests::*;
use serde_json::json;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use loom_api::realtime::RealtimeHub;
use loom_api::server::{build_router, AppState};
use loom_api::AppCache;
use loom_core::doctype::{DocTypeRegistry, RhaiHookRunner};
use loom_core::script::{create_engine, ScriptCache};

/// Build a test Axum app with a real database and the given DocTypes.
async fn build_test_app(db: &TestDb, registry: Arc<DocTypeRegistry>) -> axum::Router {
    let pool = db.pool.clone();
    let engine = Arc::new(create_engine());
    let cache = ScriptCache::new();
    let hook_runner = Arc::new(RhaiHookRunner::new(engine, cache));
    let realtime = RealtimeHub::new();
    let app_cache = AppCache::new();

    let state = AppState {
        pool,
        registry,
        hook_runner,
        realtime,
        cache: app_cache,
    };

    build_router(state, None)
}

/// Helper to read response body as JSON.
async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(json!(null))
}

/// Helper: login and return the session cookie for subsequent requests.
async fn login_as_admin(app: &mut axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "email": "Administrator", "password": "admin" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

#[tokio::test]
async fn test_doctype_meta_endpoint() {
    skip_without_db!();
    let meta = test_doctype("Api Meta DT");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/doctype/Api%20Meta%20DT")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let data = body.get("data").expect("response should have 'data' key");
    assert_eq!(
        data.get("name").and_then(|v| v.as_str()),
        Some("Api Meta DT")
    );
    // Should include fields array
    let fields = data.get("fields").and_then(|v| v.as_array());
    assert!(fields.is_some(), "meta should contain 'fields' array");
    assert!(
        !fields.unwrap().is_empty(),
        "fields array should not be empty"
    );
}

#[tokio::test]
async fn test_session_endpoint_returns_roles() {
    skip_without_db!();
    let meta = test_doctype("Api SessRoles");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(
        body.get("user").and_then(|v| v.as_str()),
        Some("Administrator")
    );
    // Should include roles array
    let roles = body.get("roles").and_then(|v| v.as_array());
    assert!(roles.is_some(), "session should contain 'roles' array");
    let roles = roles.unwrap();
    assert!(
        roles.iter().any(|r| r.as_str() == Some("Administrator")),
        "roles should contain 'Administrator'"
    );
}

// Note: signup tests require the User DocType table (core DocType) to be migrated,
// which needs the full app install flow. Signup is tested end-to-end in the real app.

#[tokio::test]
async fn test_submit_via_api() {
    skip_without_db!();
    let meta = test_submittable_doctype("Api Submit");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    // Insert a document
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/resource/Api%20Submit")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(json!({ "title": "Submit Me" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    let id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .expect("insert should return id");

    // Submit via POST /api/resource/:doctype/:name/submit
    let submit_uri = format!("/api/resource/Api%20Submit/{}/submit", id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&submit_uri)
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "submit should succeed");
    let body = body_json(response).await;
    let docstatus = body
        .get("data")
        .and_then(|d| d.get("docstatus"))
        .and_then(|v| v.as_i64());
    assert_eq!(docstatus, Some(1), "docstatus should be 1 after submit");
}

#[tokio::test]
async fn test_cancel_via_api() {
    skip_without_db!();
    let meta = test_submittable_doctype("Api Cancel");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    // Insert a document
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/resource/Api%20Cancel")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(json!({ "title": "Cancel Me" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    let id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .expect("insert should return id");

    // Submit first
    let submit_uri = format!("/api/resource/Api%20Cancel/{}/submit", id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&submit_uri)
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Cancel via POST /api/resource/:doctype/:name/cancel
    let cancel_uri = format!("/api/resource/Api%20Cancel/{}/cancel", id);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&cancel_uri)
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "cancel should succeed");
    let body = body_json(response).await;
    let docstatus = body
        .get("data")
        .and_then(|d| d.get("docstatus"))
        .and_then(|v| v.as_i64());
    assert_eq!(docstatus, Some(2), "docstatus should be 2 after cancel");
}

#[tokio::test]
async fn test_settings_crud() {
    skip_without_db!();
    let meta = test_doctype("Api Settings");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    // PUT /api/settings/test_key
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/settings/test_key")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(
                    json!({ "theme": "dark", "language": "en" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // GET /api/settings/test_key
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings/test_key")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let data = body.get("data");
    assert!(data.is_some(), "GET settings should return data");
    let data = data.unwrap();
    assert_eq!(data.get("theme").and_then(|v| v.as_str()), Some("dark"));
    assert_eq!(data.get("language").and_then(|v| v.as_str()), Some("en"));
}

#[tokio::test]
async fn test_activity_endpoint() {
    skip_without_db!();
    let meta = test_doctype("Api Activity");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    // Insert a document (this should log "Created" activity)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/resource/Api%20Activity")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(json!({ "title": "Activity Test" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    let id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .expect("insert should return id");

    // Update the document (this should log "Updated" activity)
    let update_uri = format!("/api/resource/Api%20Activity/{}", id);
    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&update_uri)
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(
                    json!({ "title": "Activity Updated" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // GET /api/activity/:doctype/:name
    let activity_uri = format!("/api/activity/Api%20Activity/{}", id);
    let response = app
        .oneshot(
            Request::builder()
                .uri(&activity_uri)
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let data = body.get("data").and_then(|d| d.as_array());
    assert!(data.is_some(), "activity should return data array");
    let timeline = data.unwrap();
    assert!(
        timeline.len() >= 2,
        "should have at least 2 activity entries (Created + Updated), got {}",
        timeline.len()
    );

    // Most recent should be "Updated"
    assert_eq!(
        timeline[0].get("action").and_then(|v| v.as_str()),
        Some("Updated")
    );
}
