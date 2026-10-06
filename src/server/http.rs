use std::io::{Error, ErrorKind};
use std::sync::Arc;

use http::{Method, StatusCode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::Sender;

use super::app::{self, AppResponse};
use crate::TransformConfig;

const MAX_HEADER_BYTES: usize = 64 * 1024;
const READ_CHUNK_SIZE: usize = 4096;
const MAX_HEADERS: usize = 64;

#[derive(Debug)]
pub struct HttpRequest {
    pub method: Method,
    pub path: String,
    pub body: Vec<u8>,
}

#[derive(Debug)]
struct ParsedRequest {
    request: HttpRequest,
    close_after_response: bool,
}

pub async fn start_server(
    listen_addr: Option<&str>,
    ready: Option<Sender<()>>,
    docs_redirect_url: Option<String>,
    config: Arc<TransformConfig>,
) {
    let addr = listen_addr.unwrap_or("127.0.0.1:3003");
    if let Some(url) = docs_redirect_url.as_ref() {
        println!("Redirecting /docs/ to: {url}");
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind {addr}: {e}"));
    println!("Listening on: http://{addr}");
    if let Some(ready) = ready
        && ready.send(()).await.is_err()
    {
        eprintln!("Ready channel closed before startup notification");
    }

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("Accept error: {e}");
                continue;
            }
        };

        let config = Arc::clone(&config);
        let docs_redirect_url = docs_redirect_url.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, config, docs_redirect_url).await {
                eprintln!("Connection error: {e}");
            }
        });
    }
}

async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    config: Arc<TransformConfig>,
    docs_redirect_url: Option<String>,
) -> std::io::Result<()> {
    let mut pending = Vec::with_capacity(READ_CHUNK_SIZE * 2);
    loop {
        let parsed = match read_request(&mut stream, &mut pending).await {
            Ok(Some(parsed)) => parsed,
            Ok(None) => return Ok(()),
            Err(e) if e.kind() == ErrorKind::InvalidData => {
                write_response(
                    &mut stream,
                    simple_error_response(StatusCode::BAD_REQUEST, format!("Bad request: {e}")),
                    true,
                )
                .await?;
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let response = app::route_request(
            parsed.request,
            Arc::clone(&config),
            docs_redirect_url.clone(),
        )
        .await;
        write_response(&mut stream, response, parsed.close_after_response).await?;

        if parsed.close_after_response {
            return Ok(());
        }
    }
}

async fn read_request(
    stream: &mut tokio::net::TcpStream,
    pending: &mut Vec<u8>,
) -> std::io::Result<Option<ParsedRequest>> {
    let mut read_buf = [0_u8; READ_CHUNK_SIZE];
    let header_end;

    loop {
        if let Some(end) = find_header_end(pending) {
            header_end = end;
            break;
        }

        if pending.len() > MAX_HEADER_BYTES {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "request headers exceed limit",
            ));
        }

        let n = stream.read(&mut read_buf).await?;
        if n == 0 {
            if pending.is_empty() {
                return Ok(None);
            }
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "connection closed mid-request",
            ));
        }
        pending.extend_from_slice(&read_buf[..n]);
    }

    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut request = httparse::Request::new(&mut headers);
    let header_slice = &pending[..header_end];
    match request.parse(header_slice) {
        Ok(httparse::Status::Complete(_)) => {}
        Ok(httparse::Status::Partial) => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "incomplete request headers",
            ));
        }
        Err(e) => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid request headers: {e}"),
            ));
        }
    }

    let method = request
        .method
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing HTTP method"))
        .and_then(|m| {
            Method::from_bytes(m.as_bytes()).map_err(|e| {
                Error::new(ErrorKind::InvalidData, format!("invalid HTTP method: {e}"))
            })
        })?;

    let raw_path = request
        .path
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing request path"))?;
    let (path, _) = split_path_query(raw_path);
    let path = path.to_string();
    let close_after_response = should_close_connection(request.version, request.headers);

    let content_length = parse_content_length(request.headers)?;
    let total_len = header_end
        .checked_add(content_length)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "request body too large"))?;
    while pending.len() < total_len {
        let n = stream.read(&mut read_buf).await?;
        if n == 0 {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "connection closed before body completed",
            ));
        }
        pending.extend_from_slice(&read_buf[..n]);
    }

    let body = pending[header_end..total_len].to_vec();
    pending.drain(..total_len);

    Ok(Some(ParsedRequest {
        request: HttpRequest { method, path, body },
        close_after_response,
    }))
}

fn should_close_connection(version: Option<u8>, headers: &[httparse::Header<'_>]) -> bool {
    let mut connection_tokens = Vec::new();
    for header in headers {
        if header.name.eq_ignore_ascii_case("connection")
            && let Ok(value) = std::str::from_utf8(header.value)
        {
            connection_tokens.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .map(str::to_ascii_lowercase)
                    .filter(|token| !token.is_empty()),
            );
        }
    }

    match version {
        Some(1) => connection_tokens.iter().any(|token| token == "close"),
        Some(0) => !connection_tokens.iter().any(|token| token == "keep-alive"),
        _ => true,
    }
}

fn parse_content_length(headers: &[httparse::Header<'_>]) -> std::io::Result<usize> {
    let mut content_length = None;
    for header in headers {
        if header.name.eq_ignore_ascii_case("transfer-encoding")
            && !header.value.eq_ignore_ascii_case(b"identity")
        {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "unsupported transfer-encoding",
            ));
        }

        if header.name.eq_ignore_ascii_case("content-length") {
            let value = std::str::from_utf8(header.value).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid content-length header encoding: {e}"),
                )
            })?;
            let parsed = value.trim().parse::<usize>().map_err(|e| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid content-length header value: {e}"),
                )
            })?;
            if let Some(existing) = content_length {
                if existing != parsed {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "conflicting content-length headers",
                    ));
                }
            } else {
                content_length = Some(parsed);
            }
        }
    }
    Ok(content_length.unwrap_or(0))
}

fn split_path_query(path: &str) -> (&str, Option<&str>) {
    match path.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (path, None),
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|idx| idx + 4)
}

fn simple_error_response(status: StatusCode, message: String) -> AppResponse {
    let mut response = AppResponse::new(message.into_bytes());
    *response.status_mut() = status;
    response.headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/plain"),
    );
    response
}

async fn write_response(
    stream: &mut tokio::net::TcpStream,
    response: AppResponse,
    close_after_response: bool,
) -> std::io::Result<()> {
    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown");
    let body = response.body();

    let mut out = Vec::with_capacity(256 + body.len());
    out.extend_from_slice(format!("HTTP/1.1 {} {}\r\n", status.as_u16(), reason).as_bytes());
    for (name, value) in response.headers() {
        out.extend_from_slice(name.as_str().as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    if close_after_response {
        out.extend_from_slice(b"Connection: close\r\n");
    } else {
        out.extend_from_slice(b"Connection: keep-alive\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    stream.write_all(&out).await
}
