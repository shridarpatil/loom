use loom_core::script::{create_engine, ScriptCache};

#[tokio::test]
async fn test_cache_miss_compiles() {
    let engine = create_engine();
    let cache = ScriptCache::new();

    let result = cache
        .get_or_compile("test_script", &engine, "fn hello() { 42 }")
        .await;

    assert!(
        result.is_ok(),
        "First compilation should succeed: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_cache_hit_returns_cached() {
    let engine = create_engine();
    let cache = ScriptCache::new();

    let source = "fn greet(name) { \"Hello, \" + name }";

    // First call — compiles and caches
    let _ast1 = cache
        .get_or_compile("greet_script", &engine, source)
        .await
        .expect("first compile should succeed");

    // Second call — should return cached AST (same key)
    let ast2 = cache
        .get_or_compile("greet_script", &engine, source)
        .await
        .expect("second call should succeed from cache");

    // Both ASTs should be functionally equivalent. We verify by checking
    // that the function can be called from the second AST as well.
    let mut scope = rhai::Scope::new();
    let result: String = engine
        .call_fn(&mut scope, &ast2, "greet", ("World",))
        .expect("calling function from cached AST should work");
    assert_eq!(result, "Hello, World");
}

#[tokio::test]
async fn test_cache_invalidate() {
    let engine = create_engine();
    let cache = ScriptCache::new();

    let source_v1 = "fn version() { 1 }";
    let source_v2 = "fn version() { 2 }";

    // Compile and cache v1
    cache
        .get_or_compile("ver_script", &engine, source_v1)
        .await
        .expect("v1 compile");

    // Invalidate
    cache.invalidate("ver_script").await;

    // Re-compile with v2 (since cache was invalidated, it should compile the new source)
    let ast = cache
        .get_or_compile("ver_script", &engine, source_v2)
        .await
        .expect("v2 compile after invalidate");

    let mut scope = rhai::Scope::new();
    let result: i64 = engine
        .call_fn(&mut scope, &ast, "version", ())
        .expect("calling version() from v2 AST");
    assert_eq!(result, 2, "After invalidate and recompile, should get v2");
}

#[tokio::test]
async fn test_cache_clear() {
    let engine = create_engine();
    let cache = ScriptCache::new();

    // Populate cache with multiple entries
    cache
        .get_or_compile("script_a", &engine, "fn a() { 1 }")
        .await
        .expect("compile a");
    cache
        .get_or_compile("script_b", &engine, "fn b() { 2 }")
        .await
        .expect("compile b");

    // Clear all
    cache.clear().await;

    // After clear, compiling with new source should produce the new version
    let source_a_v2 = "fn a() { 100 }";
    let ast = cache
        .get_or_compile("script_a", &engine, source_a_v2)
        .await
        .expect("compile a v2 after clear");

    let mut scope = rhai::Scope::new();
    let result: i64 = engine
        .call_fn(&mut scope, &ast, "a", ())
        .expect("calling a() from v2 AST after clear");
    assert_eq!(
        result, 100,
        "After clear and recompile, should get new version"
    );
}

#[tokio::test]
async fn test_cache_compile_error() {
    let engine = create_engine();
    let cache = ScriptCache::new();

    // Invalid Rhai script — mismatched braces
    let result = cache
        .get_or_compile("bad_script", &engine, "fn broken( { }")
        .await;

    assert!(result.is_err(), "Invalid script should return an error");

    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("compile error") || err_msg.contains("error"),
        "Error message should indicate a compilation problem: {}",
        err_msg
    );
}
