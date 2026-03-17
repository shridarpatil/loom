use loom_core::script::{create_engine, compile_script, call_function};
use loom_core::script::api::register_loom_api;

#[test]
fn test_create_engine() {
    let engine = create_engine();
    // Should compile a simple expression
    let ast = engine.compile("let x = 1 + 2; x").unwrap();
    let result: i64 = engine.eval_ast(&ast).unwrap();
    assert_eq!(result, 3);
}

#[test]
fn test_compile_script() {
    let engine = create_engine();
    let result = compile_script(&engine, "fn hello() { 42 }");
    assert!(result.is_ok());
}

#[test]
fn test_compile_invalid_script() {
    let engine = create_engine();
    let result = compile_script(&engine, "fn { broken syntax");
    assert!(result.is_err());
}

#[test]
fn test_call_function() {
    let engine = create_engine();
    let ast = compile_script(&engine, "fn add(a, b) { a + b }").unwrap();
    let result = call_function(&engine, &ast, "add", (3_i64, 4_i64));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().as_int().unwrap(), 7);
}

#[test]
fn test_loom_api_today() {
    let mut engine = create_engine();
    register_loom_api(&mut engine);
    let result: String = engine.eval("today()").unwrap();
    // Should be a date string like "2026-03-17"
    assert_eq!(result.len(), 10);
    assert!(result.contains('-'));
}

#[test]
fn test_loom_api_now() {
    let mut engine = create_engine();
    register_loom_api(&mut engine);
    let result: String = engine.eval("now()").unwrap();
    // Should be an ISO datetime
    assert!(result.len() > 10);
    assert!(result.contains('T'));
}

#[test]
fn test_loom_api_date_diff() {
    let mut engine = create_engine();
    register_loom_api(&mut engine);
    let result: i64 = engine.eval(r#"date_diff("2026-03-20", "2026-03-15")"#).unwrap();
    assert_eq!(result, 5);
}

#[test]
fn test_loom_api_throw() {
    let mut engine = create_engine();
    register_loom_api(&mut engine);
    let result = engine.eval::<()>(r#"throw("test error")"#);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("test error"));
}

#[test]
fn test_sandbox_limits() {
    let engine = create_engine();
    // Infinite loop should be caught by operation limit
    let result = engine.eval::<()>("loop { }");
    assert!(result.is_err());
}
