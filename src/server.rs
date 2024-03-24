use axum::{
    body::Body,
    http::Response,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use tokio::fs;

use crate::{transform_str, TransformConfig};

async fn transform(input: String) -> impl IntoResponse {
    transform_str(input, &TransformConfig::default())
        .map(|output| {
            Response::builder()
                .header("Content-Type", "image/svg+xml")
                .body(Body::from(output))
                .unwrap()
        })
        .map_err(|e| {
            // TODO: make the error more informative, e.g. by returning a JSON object
            // including line number(s) of failed elements.
            Response::builder()
                .status(400)
                .header("Content-Type", "text/plain")
                .body(Body::from(format!("Error: {}", e)))
                .unwrap()
        })
}

async fn index() -> Html<String> {
    // Perhaps in production builds this should cache on first use.
    // During development it's useful to have it re-read each request.
    let content = fs::read_to_string("static/index.html").await.unwrap();
    Html(content)
}

pub async fn start_server(listen_addr: Option<&str>) {
    let addr = listen_addr.unwrap_or("127.0.0.1:3003");
    let app = Router::new()
        .route("/", get(index))
        .route("/transform", post(transform));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Listening on: http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
