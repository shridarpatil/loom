use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

/// Multi-tenant site resolution middleware.
/// Resolves the site from the `Host` header or `X-Loom-Site` header.
pub async fn site_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // TODO: Resolve site from headers, set in request extensions
    let _site = request
        .headers()
        .get("X-Loom-Site")
        .and_then(|v| v.to_str().ok())
        .or_else(|| request.headers().get("Host").and_then(|v| v.to_str().ok()))
        .unwrap_or("localhost");

    let response = next.run(request).await;
    Ok(response)
}
