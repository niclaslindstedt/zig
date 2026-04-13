//! Static file handler for the embedded React web UI.
//!
//! The React app lives in `../web/` and is compiled to `../web/dist/` by
//! `npm run build`. `rust-embed` walks that directory at compile time and
//! bakes every file into the `zig-serve` crate so the web UI ships as part
//! of the binary — no filesystem required at runtime.
//!
//! Routes are only mounted when `ServeConfig.web` is true. Unknown paths
//! fall back to `index.html` so client-side routing works.

use axum::body::Body;
use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../web/dist/"]
#[exclude = ".gitkeep"]
struct WebAssets;

/// Handler for `GET /` — always returns the SPA entry point.
pub async fn index() -> Response {
    serve_asset("index.html")
}

/// Handler for `GET /{*path}` — serves static assets; falls back to
/// `index.html` for client-side routes.
pub async fn asset(Path(path): Path<String>) -> Response {
    // Guard against any attempt to read the API namespace through this
    // catch-all handler. Axum route precedence should already prevent this,
    // but we belt-and-brace it.
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    }
    if WebAssets::get(&path).is_some() {
        serve_asset(&path)
    } else {
        serve_asset("index.html")
    }
}

fn serve_asset(path: &str) -> Response {
    match WebAssets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(file.data.into_owned()))
                .unwrap()
        }
        None => (
            StatusCode::NOT_FOUND,
            "zig web bundle is not built — run `make web-build` and rebuild zig",
        )
            .into_response(),
    }
}
