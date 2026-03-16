use rhai::{Engine, Scope, AST};

use crate::error::{LoomError, LoomResult};

/// Create a sandboxed Rhai engine with Loom API functions registered.
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();

    // Sandbox: disable all potentially dangerous operations
    engine.set_max_expr_depths(64, 64);
    engine.set_max_call_levels(32);
    engine.set_max_operations(100_000);
    engine.set_max_string_size(1_000_000);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);

    engine
}

/// Compile a Rhai script into an AST.
pub fn compile_script(engine: &Engine, script: &str) -> LoomResult<AST> {
    engine
        .compile(script)
        .map_err(|e| LoomError::Script(format!("Compilation error: {}", e)))
}

/// Execute a function from a compiled Rhai AST.
pub fn call_function(
    engine: &Engine,
    ast: &AST,
    function_name: &str,
    args: impl rhai::FuncArgs,
) -> LoomResult<rhai::Dynamic> {
    let mut scope = Scope::new();
    engine
        .call_fn(&mut scope, ast, function_name, args)
        .map_err(|e| LoomError::Script(format!("Runtime error in '{}': {}", function_name, e)))
}
