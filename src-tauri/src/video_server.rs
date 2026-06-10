use axum::{
    body::Body,
    extract::Query,
    http::{header, HeaderMap, Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::AsyncSeekExt;
use tower_http::cors::CorsLayer;

pub fn start() -> u16 {
    let (tx, rx) = std::sync::mpsc::channel();

    tauri::async_runtime::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("video server: bind failed");

        let port = listener.local_addr().unwrap().port();
        let _ = tx.send(port);

        let router = Router::new()
            .route("/file", get(serve_file))
            .layer(CorsLayer::permissive());

        axum::serve(listener, router)
            .await
            .expect("video server: serve failed");
    });

    rx.recv().expect("video server: port channel closed")
}

async fn serve_file(
    Query(params): Query<HashMap<String, String>>,
    req_headers: HeaderMap,
) -> Response<Body> {
    let path_str = match params.get("path") {
        Some(p) => urlencoding::decode(p).unwrap_or_default().into_owned(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let path = PathBuf::from(&path_str);

    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let file_size = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mime = mime_guess::from_path(&path)
        .first_or_octet_stream()
        .to_string();

    let range = req_headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_range(v, file_size));

    if let Some((start, end)) = range {
        if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
            return StatusCode::RANGE_NOT_SATISFIABLE.into_response();
        }

        let len = end - start + 1;
        use tokio::io::AsyncReadExt;
        let limited = file.take(len);
        let stream = tokio_util::io::ReaderStream::new(limited);

        Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, len)
            .header(
                header::CONTENT_RANGE,
                format!("bytes {}-{}/{}", start, end, file_size),
            )
            .header(header::ACCEPT_RANGES, "bytes")
            .body(Body::from_stream(stream))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
        let stream = tokio_util::io::ReaderStream::new(file);

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, file_size)
            .header(header::ACCEPT_RANGES, "bytes")
            .body(Body::from_stream(stream))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    }
}

fn parse_range(range: &str, file_size: u64) -> Option<(u64, u64)> {
    let range = range.strip_prefix("bytes=")?;
    let mut parts = range.splitn(2, '-');
    let start: u64 = parts.next()?.parse().ok()?;
    let end_str = parts.next()?;
    let end: u64 = if end_str.is_empty() {
        file_size.saturating_sub(1)
    } else {
        end_str.parse().ok()?
    };
    if start > end || end >= file_size {
        return None;
    }
    Some((start, end))
}
