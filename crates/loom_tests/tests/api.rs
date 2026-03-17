//! Integration tests for HTTP API endpoints via Axum test utilities.

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

#[tokio::test]
async fn test_health_endpoint() {
    skip_without_db!();
    let meta = test_doctype("Api Health");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let app = build_test_app(&db, registry).await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_login_endpoint() {
    skip_without_db!();
    let meta = test_doctype("Api Login");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let app = build_test_app(&db, registry).await;

    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(
        body.get("user").and_then(|v| v.as_str()),
        Some("Administrator")
    );
}

#[tokio::test]
async fn test_login_wrong_password() {
    skip_without_db!();
    let meta = test_doctype("Api Bad Login");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let app = build_test_app(&db, registry).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "email": "Administrator", "password": "wrong" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_route_requires_auth() {
    skip_without_db!();
    let meta = test_doctype("Api Auth Req");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let app = build_test_app(&db, registry).await;

    // Access protected route without auth
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/resource/Api%20Auth%20Req")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
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

    // Extract Set-Cookie header
    response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

#[tokio::test]
async fn test_crud_via_api() {
    skip_without_db!();
    let meta = test_doctype("Api Crud");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    // INSERT
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/resource/Api%20Crud")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(json!({ "title": "API Created" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // INSERT returns 201 Created
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::CREATED,
        "Expected 200 or 201 for insert, got {}",
        response.status()
    );
    let body = body_json(response).await;
    let id = body
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(!id.is_empty());

    // GET
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&format!("/api/resource/Api%20Crud/{}", id))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(
        body.get("data")
            .and_then(|d| d.get("title"))
            .and_then(|v| v.as_str()),
        Some("API Created")
    );

    // UPDATE
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/resource/Api%20Crud/{}", id))
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(json!({ "title": "API Updated" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // DELETE
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/resource/Api%20Crud/{}", id))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // GET after delete should 404
    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/resource/Api%20Crud/{}", id))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_list_via_api() {
    skip_without_db!();
    let meta = test_doctype("Api List");
    let (db, registry) = TestDb::with_doctypes(vec![meta]).await;
    let mut app = build_test_app(&db, registry).await;

    let cookie = login_as_admin(&mut app).await;

    // Insert 3 docs
    for i in 1..=3 {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/resource/Api%20List")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie)
                    .body(Body::from(
                        json!({ "title": format!("List Item {}", i) }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    // Get list
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/resource/Api%20List?limit=10")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let data = body.get("data").and_then(|d| d.as_array()).unwrap();
    assert_eq!(data.len(), 3);
}

#[tokio::test]
async fn test_session_endpoint() {
    skip_without_db!();
    let meta = test_doctype("Api Session");
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
}
