//! Controlled fixture for manual end-to-end tests and timing comparisons.
use axum::extract::Query;
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Response},
    routing::get,
};
async fn file(
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response<Body> {
    let total = params
        .get("bytes")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(8 * 1024 * 1024)
        .clamp(1, 64 * 1024 * 1024 * 1024);
    let (start, end, ranged) = headers
        .get("range")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("bytes="))
        .and_then(|h| h.split_once('-'))
        .map(|(a, b)| {
            (
                a.parse::<usize>().unwrap(),
                b.parse::<usize>().unwrap().min(total - 1),
                true,
            )
        })
        .unwrap_or((0, total - 1, false));
    if start > end || start >= total {
        return Response::builder()
            .status(416)
            .header("Content-Range", format!("bytes */{total}"))
            .body(Body::empty())
            .unwrap();
    }
    let mut response = Response::builder()
        .status(if ranged { 206 } else { 200 })
        .header("ETag", "\"fixture-v1\"");
    if ranged {
        response = response.header("Content-Range", format!("bytes {start}-{end}/{total}"));
    }
    let slow = params.contains_key("slow");
    {
        let stream =
            futures_util::stream::unfold((start, end, slow), |(offset, end, slow)| async move {
                if offset > end {
                    return None;
                }
                if slow {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                let next = (offset + 65536).min(end + 1);
                let chunk: Vec<u8> = (offset..next).map(|i| (i % 251) as u8).collect();
                Some((Ok::<_, std::io::Error>(chunk), (next, end, slow)))
            });
        response
            .header("Content-Length", (end - start + 1).to_string())
            .body(Body::from_stream(stream))
            .unwrap()
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18765").await?;
    println!("http://127.0.0.1:18765/fixture.bin");
    axum::serve(listener, Router::new().route("/fixture.bin", get(file))).await?;
    Ok(())
}
