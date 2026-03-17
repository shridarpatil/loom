use std::sync::Arc;

use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::{self, Next},
    response::{Html, IntoResponse},
    routing::{delete, get, post, put},
    Json, Router,
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use loom_core::doctype::{DocTypeRegistry, RhaiHookRunner};

use crate::cache::AppCache;
use crate::middleware::auth;
use crate::realtime::{self, RealtimeHub};
use crate::routes::{
    self, activity, config, customize, doctype, method, resource, settings, upload,
};

/// Shared application state accessible by all handlers.
#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub registry: Arc<DocTypeRegistry>,
    pub hook_runner: Arc<RhaiHookRunner>,
    pub realtime: RealtimeHub,
    pub cache: AppCache,
}

/// Build the Axum router with all API routes.
pub fn build_router(state: AppState, frontend_dir: Option<String>) -> Router {
    // Build auth middleware that captures state
    let auth_state = state.clone();
    let auth_layer = middleware::from_fn(move |req: Request, next: Next| {
        let s = auth_state.clone();
        async move { auth::inject_context(s, req, next).await }
    });

    // Auth routes — public, no auth middleware
    let public = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/api/auth/logout", post(routes::auth::logout))
        .route("/api/auth/signup", post(routes::auth::signup))
        // Theme config is public (read-only)
        .route("/api/config/theme", get(config::get_theme))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    // Protected routes — require auth
    let protected = Router::new()
        // Resource CRUD routes
        .route("/api/resource/:doctype", get(resource::get_list))
        .route("/api/resource/:doctype", post(resource::insert_doc))
        .route("/api/resource/:doctype/:name", get(resource::get_doc))
        .route("/api/resource/:doctype/:name", put(resource::update_doc))
        .route("/api/resource/:doctype/:name", delete(resource::delete_doc))
        // Submit / Cancel
        .route(
            "/api/resource/:doctype/:name/submit",
            post(resource::submit_doc),
        )
        .route(
            "/api/resource/:doctype/:name/cancel",
            post(resource::cancel_doc),
        )
        // DocType meta
        .route("/api/doctype/:name", get(doctype::get_meta))
        .route("/api/doctype/:name/export", post(doctype::export_meta))
        // Customization
        .route("/api/customize/:doctype", get(customize::get_customization))
        .route(
            "/api/customize/:doctype",
            put(customize::save_customization),
        )
        .route(
            "/api/customize/:doctype/export",
            post(customize::export_customization),
        )
        // Apps
        .route("/api/apps", get(doctype::list_apps_handler))
        // Session
        .route("/api/session", get(session_info))
        // Whitelisted methods
        .route("/api/method/:path", post(method::call_method))
        // Theme config (admin write)
        .route("/api/config/theme", put(config::save_theme))
        // User settings
        .route("/api/settings/:key", get(settings::get_setting))
        .route("/api/settings/:key", put(settings::save_setting))
        // Sidebar
        .route("/api/sidebar", get(settings::get_sidebar))
        // Plugin pages
        .route("/api/plugins/pages", get(settings::get_plugin_pages))
        // Role Permission Manager
        .route(
            "/api/role-permission/:doctype",
            get(customize::get_role_permissions),
        )
        .route(
            "/api/role-permission/:doctype",
            put(customize::save_role_permissions),
        )
        .route(
            "/api/role-permission/:doctype",
            delete(customize::reset_role_permissions),
        )
        .route(
            "/api/role-permission-by-role/:role",
            get(customize::get_permissions_by_role),
        )
        // Activity / audit trail
        .route("/api/activity/:doctype/:name", get(activity::get_activity))
        .route(
            "/api/activity/:doctype/:name/comment",
            post(activity::add_comment),
        )
        // File uploads
        .route("/api/upload", post(upload::upload_file))
        .layer(auth_layer)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    // WebSocket route — outside auth middleware (WS handles its own auth)
    let ws_routes = Router::new()
        .route("/ws", get(realtime::ws_handler))
        .with_state(state);

    // Serve uploaded files
    let uploads_dir = std::path::Path::new("sites/uploads");
    let upload_service = Router::new().nest_service("/uploads", ServeDir::new(uploads_dir));

    let api = public
        .merge(protected)
        .merge(ws_routes)
        .merge(upload_service);

    // If a frontend directory is provided, serve static assets from it
    // and fall back to index.html for SPA routes
    if let Some(ref dir) = frontend_dir {
        let spa_dir = dir.clone();
        api.fallback_service(ServeDir::new(dir).fallback(get(move || {
            let d = spa_dir.clone();
            async move { serve_index(&d).await }
        })))
    } else {
        api
    }
}

/// Read and serve index.html for SPA client-side routing fallback
async fn serve_index(dir: &str) -> impl IntoResponse {
    let index_path = std::path::Path::new(dir).join("index.html");
    match tokio::fs::read_to_string(&index_path).await {
        Ok(html) => Html(html).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Frontend not found").into_response(),
    }
}

async fn health_check() -> &'static str {
    "ok"
}

async fn session_info(
    Extension(ctx): Extension<loom_core::context::RequestContext>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "user": ctx.user,
        "roles": ctx.roles,
    }))
}
