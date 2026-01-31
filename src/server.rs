use axum::{
    body::Body,
    extract::{Path, Query},
    http::Response,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde_derive::Deserialize;
use tokio::sync::mpsc::Sender;

use crate::errors::Error;
use crate::json_api::{TransformRequest, TransformResponse, JSON_API_VERSION};
use crate::{transform_str, TransformConfig};

// Content-Security-Policy - allow inline CSS used for the generated SVG images,
// but otherwise restrict to same-origin resources.
// Includes 'wasm-unsafe-eval' for consistency even though is doing server-side
// transforms rather than in-browser.
// img-src requires `blob:` scheme for clipboard copy.
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' blob:; frame-ancestors 'none'";

// Not all fields make sense for the editor, but add_metadata
// is needed to allow hover-over line highlighting.
#[derive(Debug, Default, Deserialize)]
struct RequestConfig {
    #[serde(default)]
    add_metadata: bool,
    #[serde(default)]
    format: ResponseFormat,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ResponseFormat {
    #[default]
    Json,
    Raw,
}

impl From<RequestConfig> for TransformConfig {
    fn from(config: RequestConfig) -> Self {
        TransformConfig {
            add_metadata: config.add_metadata,
            ..Default::default()
        }
    }
}

async fn transform(config: Query<RequestConfig>, input: String) -> impl IntoResponse {
    let Query(config) = config;

    match config.format {
        // for JSON format the config is included in the JSON request body
        ResponseFormat::Json => transform_json_handler(input),
        ResponseFormat::Raw => transform_raw_handler(input, config),
    }
}

fn transform_json_handler(input: String) -> Response<Body> {
    let response: TransformResponse = match serde_json::from_str::<TransformRequest>(&input) {
        Ok(request) => {
            if request.version != JSON_API_VERSION {
                TransformResponse::error(format!(
                    "Unsupported API version: {} (expected {})",
                    request.version, JSON_API_VERSION
                ))
            } else {
                let config: TransformConfig = request.config.into();
                match transform_str(request.input, &config) {
                    Ok(svg) => {
                        if svg.is_empty() {
                            TransformResponse::error("empty response".to_string())
                        } else {
                            TransformResponse::success(svg)
                        }
                    }
                    Err(e) => TransformResponse::error(e.to_string()),
                }
            }
        }
        Err(e) => TransformResponse::error(format!("Invalid JSON request: {e}")),
    };

    let is_error = response.error.is_some();
    let body = serde_json::to_string(&response).expect("Failed to serialize response");

    let mut builder = Response::builder()
        .header("Content-Type", "application/json")
        .header("Content-Security-Policy", CSP);

    if is_error {
        builder = builder.status(400);
    }

    builder.body(Body::from(body)).unwrap()
}

fn transform_raw_handler(input: String, config: RequestConfig) -> Response<Body> {
    transform_str(input, &config.into())
        .and_then(|output| {
            if output.is_empty() {
                Err(Error::Document("empty response".into()))
            } else {
                Ok(output)
            }
        })
        .map(|output| {
            Response::builder()
                .header("Content-Type", "image/svg+xml")
                .header("Content-Security-Policy", CSP)
                .body(Body::from(output))
                .unwrap()
        })
        .unwrap_or_else(|e| {
            // TODO: make the error more informative, e.g. by returning a JSON object
            // including line number(s) of failed elements.
            Response::builder()
                .status(400)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Error: {e}")))
                .unwrap()
        })
}

macro_rules! include_or_read {
    ($path:expr, $mime:expr) => {{
        // If configured as a release build, use include_bytes! to embed the file.
        #[cfg(not(debug_assertions))]
        let content =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path)).as_ref();
        // During development it's useful to have it re-read each request.
        #[cfg(debug_assertions)]
        let content = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path))
            .await
            .unwrap();

        Response::builder()
            .header("Content-Type", $mime)
            .header("Content-Security-Policy", CSP)
            .body(Body::from(content))
            .unwrap()
    }};
}

macro_rules! include_js {
    ($path:expr) => {{
        include_or_read!($path, "application/javascript")
    }};
}

macro_rules! include_css {
    ($path:expr) => {{
        include_or_read!($path, "text/css")
    }};
}

macro_rules! include_html {
    ($path:expr) => {{
        include_or_read!($path, "text/html")
    }};
}

macro_rules! include_ico {
    ($path:expr) => {{
        include_or_read!($path, "image/x-icon")
    }};
}

async fn index() -> impl IntoResponse {
    include_html!("index.html")
}

async fn favicon() -> impl IntoResponse {
    include_ico!("favicon.ico")
}

// Note svgdx-server injects a different bootstrap script (-server.js) vs the bootstrap
// picked up by a static file server (such as `python3 -m http.server`). This is to ensure
// transform requests come to the server rather than being handled by the browser WASM code.
async fn bootstrap() -> impl IntoResponse {
    include_js!("svgdx-bootstrap-server.js")
}

async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    match path.as_str() {
        // Main editor files
        "main.js" => include_js!("static/main.js"),
        "svgdx-editor.css" => include_css!("static/svgdx-editor.css"),
        // Modules
        "modules/config.js" => include_js!("static/modules/config.js"),
        "modules/storage.js" => include_js!("static/modules/storage.js"),
        "modules/dom.js" => include_js!("static/modules/dom.js"),
        "modules/editor-adapter.js" => include_js!("static/modules/editor-adapter.js"),
        "modules/transform.js" => include_js!("static/modules/transform.js"),
        "modules/tabs.js" => include_js!("static/modules/tabs.js"),
        "modules/layout.js" => include_js!("static/modules/layout.js"),
        "modules/viewport.js" => include_js!("static/modules/viewport.js"),
        "modules/splitter.js" => include_js!("static/modules/splitter.js"),
        "modules/statusbar.js" => include_js!("static/modules/statusbar.js"),
        "modules/clipboard.js" => include_js!("static/modules/clipboard.js"),
        // CodeMirror vendor files
        "vendor/cm5/codemirror.min.css" => {
            include_css!("static/vendor/cm5/codemirror.min.css")
        }
        "vendor/cm5/codemirror.min.js" => include_js!("static/vendor/cm5/codemirror.min.js"),
        "vendor/cm5/mode/xml/xml.min.js" => include_js!("static/vendor/cm5/mode/xml/xml.min.js"),
        "vendor/cm5/addon/fold/xml-fold.min.js" => {
            include_js!("static/vendor/cm5/addon/fold/xml-fold.min.js")
        }
        "vendor/cm5/addon/fold/foldcode.js" => {
            include_js!("static/vendor/cm5/addon/fold/foldcode.js")
        }
        "vendor/cm5/addon/fold/foldgutter.js" => {
            include_js!("static/vendor/cm5/addon/fold/foldgutter.js")
        }
        "vendor/cm5/addon/fold/foldgutter.min.css" => {
            include_css!("static/vendor/cm5/addon/fold/foldgutter.min.css")
        }
        "vendor/cm5/addon/display/autorefresh.min.js" => {
            include_js!("static/vendor/cm5/addon/display/autorefresh.min.js")
        }
        _ => Response::builder()
            .status(404)
            .header("Content-Type", "text/plain")
            .body(Body::from("File not found"))
            .unwrap(),
    }
}

pub async fn start_server(listen_addr: Option<&str>, ready: Option<Sender<()>>) {
    let addr = listen_addr.unwrap_or("127.0.0.1:3003");
    let app = Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(favicon))
        .route("/static/{*path}", get(static_file))
        .route("/svgdx-bootstrap.js", get(bootstrap))
        .route("/api/transform", post(transform));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on: http://{addr}");
    if let Some(ready) = ready {
        ready.send(()).await.unwrap();
    }
    axum::serve(listener, app).await.unwrap();
}
