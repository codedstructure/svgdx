use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Path, Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use hyper::StatusCode;
use serde_derive::Deserialize;
use tokio::sync::mpsc::{Sender, channel};

use crate::errors::Error;
use crate::json_api::{TransformResponse, transform_json_impl};
use crate::{TransformConfig, VERSION, transform_str};

mod args;

pub use args::{Args, CliAction, parse_args, usage};

// Content-Security-Policy - allow inline CSS used for the generated SVG images,
// but otherwise restrict to same-origin resources.
// Includes 'wasm-unsafe-eval' for consistency even though is doing server-side
// transforms rather than in-browser.
// img-src requires `blob:` scheme for clipboard copy.
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' blob:; frame-ancestors 'none'";

// Not all fields make sense for the editor, but add_metadata
// is needed to allow hover-over line highlighting.
#[derive(Debug, Default, Deserialize)]
struct RequestParams {
    #[serde(default)]
    add_metadata: bool,
}

impl From<RequestParams> for TransformConfig {
    fn from(params: RequestParams) -> Self {
        TransformConfig {
            add_metadata: params.add_metadata,
            ..Default::default()
        }
    }
}

impl Args {
    fn socket_addr(&self) -> String {
        if self.address.is_ipv6() {
            format!("[{}]:{}", self.address, self.port)
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}

pub async fn run(config: CliAction, program_name: &str) {
    match config {
        CliAction::Help => {
            println!("{}", usage(program_name));
        }
        CliAction::Version => {
            println!("{program_name} v{VERSION}");
        }
        CliAction::Run(args) => {
            let address = args.socket_addr();
            let mut tx = None;
            if args.open {
                let (ch_tx, mut rx) = channel(1);
                tx = Some(ch_tx);
                let address = address.clone();
                tokio::spawn(async move {
                    if rx.recv().await.is_some() {
                        open::that(format!("http://{address}"))
                            .unwrap_or_else(|e| eprintln!("Failed to open browser: {e}"));
                    }
                });
            }
            start_server(
                Some(&address),
                tx,
                args.docs_redirect_url.clone(),
                Arc::new(args.config.into()),
            )
            .await;
        }
    }
}

async fn transform(
    State(server_config): State<Arc<TransformConfig>>,
    params: Query<RequestParams>,
    input: String,
) -> impl IntoResponse {
    let Query(params) = params;

    transform_raw_handler(input, params, &server_config)
}

async fn transform_json(
    State(config): State<Arc<TransformConfig>>,
    input: String,
) -> impl IntoResponse {
    transform_json_handler(input, &config)
}

fn transform_json_handler(input: String, config: &TransformConfig) -> Response<Body> {
    let response: TransformResponse = transform_json_impl(&input, config);

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

fn transform_raw_handler(
    input: String,
    params: RequestParams,
    server_config: &TransformConfig,
) -> Response<Body> {
    // Merge request config with server config (only `add_metadata` for raw handler)
    let mut cfg = server_config.clone();
    cfg.add_metadata = params.add_metadata;

    transform_str(input, &cfg)
        .and_then(|output| {
            if output.is_empty() {
                Err(Error::Document("empty response".into()))
            } else {
                Ok(output)
            }
        })
        .map(|output| respond_ok(output, "image/svg+xml"))
        .unwrap_or_else(|e| respond(format!("Error: {e}"), "text/plain", StatusCode::BAD_REQUEST))
}

// return 200 OK with given body / mime-type
fn respond_ok(content: impl Into<Body>, mime: &str) -> Response<Body> {
    respond(content, mime, StatusCode::OK)
}

fn respond(content: impl Into<Body>, mime: &str, status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Content-Security-Policy", CSP)
        .body(content.into())
        .unwrap()
}

macro_rules! include_or_read {
    ($path:expr, $mime:expr) => {{
        #[cfg(not(debug_assertions))]
        let content =
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path)).as_ref();
        #[cfg(debug_assertions)]
        let content = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/editor/", $path))
            .await
            .unwrap();
        respond_ok(content, $mime)
    }};
}

macro_rules! include_js {
    ($path:expr) => {{ include_or_read!($path, "application/javascript") }};
}

macro_rules! include_css {
    ($path:expr) => {{ include_or_read!($path, "text/css") }};
}

macro_rules! include_html {
    ($path:expr) => {{ include_or_read!($path, "text/html") }};
}

macro_rules! include_ico {
    ($path:expr) => {{ include_or_read!($path, "image/x-icon") }};
}

async fn index() -> impl IntoResponse {
    include_html!("index.html")
}

async fn favicon() -> impl IntoResponse {
    include_ico!("favicon.ico")
}

async fn bootstrap() -> impl IntoResponse {
    // server mode bootstrap; hardcoded here rather than loaded
    // from file to allow version string to be injected.
    let content = format!(
        r#"
console.log("svgdx: using server transform");
window.svgdx_use_server = true;
window.svgdx_version_label = "svgdx v{VERSION}";
window.dispatchEvent(new Event('svgdx-ready'));
"#
    );
    respond_ok(content, "application/javascript")
}

async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    match path.as_str() {
        "main.js" => include_js!("static/main.js"),
        "svgdx-editor.css" => include_css!("static/svgdx-editor.css"),
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
        "modules/toolbar.js" => include_js!("static/modules/toolbar.js"),
        "modules/slider.js" => include_js!("static/modules/slider.js"),
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

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "404 Not Found\n")
}

pub async fn start_server(
    listen_addr: Option<&str>,
    ready: Option<Sender<()>>,
    docs_redirect_url: Option<String>,
    config: Arc<TransformConfig>,
) {
    let addr = listen_addr.unwrap_or("127.0.0.1:3003");
    let mut app = Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(favicon))
        .route("/static/{*path}", get(static_file))
        .route("/svgdx-bootstrap.js", get(bootstrap))
        .route("/api/transform", post(transform))
        .route("/api/transform_json", post(transform_json))
        .with_state(config)
        .fallback(not_found);

    // When running locally, docs might be served from elsewhere.
    if let Some(docs_redirect_url) = docs_redirect_url {
        println!("Redirecting /docs/ to: {docs_redirect_url}");
        app = app.route(
            "/docs/",
            get(move || async move { Redirect::temporary(&docs_redirect_url) }),
        );
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on: http://{addr}");
    if let Some(ready) = ready {
        ready.send(()).await.unwrap();
    }
    axum::serve(listener, app).await.unwrap();
}
