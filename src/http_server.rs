use std::sync::Arc;

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use serde::Serialize;
use tokio::net::TcpListener;

use crate::{app::AppContext, config::AppConfig};

static WEB_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/web/dist");

pub async fn spawn(context: Arc<AppContext>) -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("failed to bind local web server")?;
    let port = listener
        .local_addr()
        .context("failed to read local web server address")?
        .port();

    let app = router(context);
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            eprintln!("[Gesto] web server error: {error:#}");
        }
    });

    Ok(port)
}

fn router(context: Arc<AppContext>) -> Router {
    Router::new()
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/status", get(get_status))
        .route("/", get(index))
        .route("/{*path}", get(asset))
        .with_state(context)
}

async fn get_config(State(context): State<Arc<AppContext>>) -> Json<AppConfig> {
    Json(context.config_snapshot())
}

async fn put_config(
    State(context): State<Arc<AppContext>>,
    Json(config): Json<AppConfig>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    context
        .save_config(config)
        .map(Json)
        .map_err(internal_error)
}

async fn get_status(State(context): State<Arc<AppContext>>) -> Json<StatusPayload> {
    Json(StatusPayload {
        server_url: context.server_url(),
        config_path: context.config_path(),
        port: context.port(),
        app_name: "Gesto".to_string(),
    })
}

async fn index() -> Response {
    serve_asset("index.html")
}

async fn asset(Path(path): Path<String>) -> Response {
    let requested = path.trim_start_matches('/');
    if requested.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }

    let candidate = if requested.is_empty() {
        "index.html"
    } else {
        requested
    };

    serve_asset(candidate)
}

fn serve_asset(path: &str) -> Response {
    let file = WEB_DIST
        .get_file(path)
        .or_else(|| WEB_DIST.get_file("index.html"));

    match file {
        Some(file) => {
            let mime = from_path(path).first_or_octet_stream();
            let mut headers = HeaderMap::new();
            let content_type = HeaderValue::from_str(mime.as_ref())
                .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
            headers.insert(header::CONTENT_TYPE, content_type);
            (StatusCode::OK, headers, file.contents()).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Web assets not built").into_response(),
    }
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusPayload {
    server_url: String,
    config_path: String,
    port: u16,
    app_name: String,
}
