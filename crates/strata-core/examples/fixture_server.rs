//! Controlled fixture for manual end-to-end tests and timing comparisons.
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Response},
    routing::get,
};
async fn file(headers: HeaderMap) -> Response<Body> {
    let total = 8 * 1024 * 1024usize;
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
    let body: Vec<u8> = (start..=end).map(|i| (i % 251) as u8).collect();
    let mut response = Response::builder()
        .status(if ranged { 206 } else { 200 })
        .header("ETag", "\"fixture-v1\"");
    if ranged {
        response = response.header("Content-Range", format!("bytes {start}-{end}/{total}"));
    }
    response.body(Body::from(body)).unwrap()
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18765").await?;
    println!("http://127.0.0.1:18765/fixture.bin");
    axum::serve(listener, Router::new().route("/fixture.bin", get(file))).await?;
    Ok(())
}
