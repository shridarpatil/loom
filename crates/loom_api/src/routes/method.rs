use std::sync::Arc;

use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use rhai::{Dynamic, Scope};
use serde_json::{json, Value};

use loom_core::context::RequestContext;

use crate::server::AppState;

/// POST /api/method/:path — call a whitelisted Rhai/WASM method.
/// The path maps to a script file: e.g., `my_app.get_leave_balance`
/// → `apps/my_app/api/get_leave_balance.rhai`
pub async fn call_method(
    State(state): State<AppState>,
    Extension(ctx): Extension<RequestContext>,
    Path(path): Path<String>,
    Json(params): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    tracing::info!("Method call: {}", path);

    // Parse path: "app_name.method_name" or just "method_name"
    let (app_name, method_name) = match path.split_once('.') {
        Some((app, method)) => (app.to_string(), method.to_string()),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Method path must be in format 'app.method_name'"})),
            ));
        }
    };

    // Look for the script file
    let script_path = format!("apps/{}/api/{}.rhai", app_name, method_name);
    let source = std::fs::read_to_string(&script_path).map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!("Method '{}' not found", path)})),
        )
    })?;

    // Create engine with full loom API registered
    let mut engine = loom_core::script::create_engine();
    loom_core::script::api::register_loom_api(&mut engine);
    loom_core::script::register_db_api(
        &mut engine,
        Arc::new(state.pool.clone()),
        state.registry.clone(),
        ctx.user.clone(),
        ctx.roles.clone(),
    );

    let ast = engine.compile(&source).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Script compile error: {}", e)})),
        )
    })?;

    // Check that the script has a `main` function
    let has_main = ast.iter_functions().any(|f| f.name == "main");
    if !has_main {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Method script must define a main(params, loom) function"})),
        ));
    }

    // Convert params to Dynamic
    let params_dynamic = rhai::serde::to_dynamic(&params).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Failed to convert params: {}", e)})),
        )
    })?;

    // Build scope with user context
    let mut scope = Scope::new();
    scope.push("__loom_user", ctx.user.clone());
    scope.push("__loom_roles", ctx.roles.clone());

    let loom_map = Dynamic::from(rhai::Map::new());

    // Execute
    let result = tokio::task::block_in_place(|| {
        engine.call_fn::<Dynamic>(&mut scope, &ast, "main", (params_dynamic, loom_map))
    });

    match result {
        Ok(returned) => {
            let result_val: Value =
                rhai::serde::from_dynamic(&returned).unwrap_or(Value::Null);
            Ok(Json(json!({ "message": result_val })))
        }
        Err(e) => {
            let err_str = e.to_string();
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": err_str})),
            ))
        }
    }
}
