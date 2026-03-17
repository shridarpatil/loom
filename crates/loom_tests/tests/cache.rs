//! Integration tests for the TTL cache from loom_api.

use loom_api::cache::AppCache;

#[tokio::test]
async fn test_session_cache_set_get() {
    let cache = AppCache::new();

    let session_data = (
        "user@example.com".to_string(),
        vec!["Admin".to_string(), "All".to_string()],
    );
    cache
        .sessions
        .set("sid-abc-123".to_string(), session_data.clone())
        .await;

    let result = cache.sessions.get("sid-abc-123").await;
    assert!(result.is_some(), "Session should be retrievable after set");

    let (email, roles) = result.unwrap();
    assert_eq!(email, "user@example.com");
    assert_eq!(roles, vec!["Admin", "All"]);
}

#[tokio::test]
async fn test_session_cache_invalidate() {
    let cache = AppCache::new();

    let session_data = ("user@example.com".to_string(), vec!["Admin".to_string()]);
    cache
        .sessions
        .set("sid-to-remove".to_string(), session_data)
        .await;

    // Verify it is set
    let before = cache.sessions.get("sid-to-remove").await;
    assert!(before.is_some(), "Session should exist before invalidation");

    // Invalidate
    cache.invalidate_session("sid-to-remove").await;

    // Verify it is gone
    let after = cache.sessions.get("sid-to-remove").await;
    assert!(after.is_none(), "Session should be gone after invalidation");
}

#[tokio::test]
async fn test_customization_cache() {
    let cache = AppCache::new();

    let overrides = Some(serde_json::json!({
        "__permissions": [
            { "role": "HR Manager", "read": true, "write": true }
        ]
    }));

    cache
        .customizations
        .set("Employee".to_string(), overrides.clone())
        .await;

    let result = cache.customizations.get("Employee").await;
    assert!(result.is_some(), "Customization should be cached");

    let cached = result.unwrap();
    assert!(cached.is_some(), "Cached value should be Some(overrides)");
    let value = cached.unwrap();
    assert!(
        value.get("__permissions").is_some(),
        "Overrides should contain __permissions"
    );

    // Invalidate and verify
    cache.invalidate_customization("Employee").await;
    let after = cache.customizations.get("Employee").await;
    assert!(
        after.is_none(),
        "Customization should be gone after invalidation"
    );
}

#[tokio::test]
async fn test_cache_independence() {
    let cache = AppCache::new();

    // Set a session
    let session_data = (
        "admin@example.com".to_string(),
        vec!["Administrator".to_string()],
    );
    cache
        .sessions
        .set("sid-independent".to_string(), session_data)
        .await;

    // Set a customization
    let overrides = Some(serde_json::json!({ "field": "value" }));
    cache
        .customizations
        .set("SomeDocType".to_string(), overrides)
        .await;

    // Invalidate session — customization should still be there
    cache.invalidate_session("sid-independent").await;

    let session_after = cache.sessions.get("sid-independent").await;
    assert!(session_after.is_none(), "Session should be invalidated");

    let custom_after = cache.customizations.get("SomeDocType").await;
    assert!(
        custom_after.is_some(),
        "Customization should survive session invalidation"
    );

    // Invalidate customization — session-related data should not be affected
    // (we already invalidated the session, but let's set a new one to verify)
    let new_session = ("other@example.com".to_string(), vec!["All".to_string()]);
    cache
        .sessions
        .set("sid-other".to_string(), new_session)
        .await;

    cache.invalidate_customization("SomeDocType").await;

    let other_session = cache.sessions.get("sid-other").await;
    assert!(
        other_session.is_some(),
        "Other session should survive customization invalidation"
    );

    let custom_gone = cache.customizations.get("SomeDocType").await;
    assert!(custom_gone.is_none(), "Customization should be invalidated");
}
