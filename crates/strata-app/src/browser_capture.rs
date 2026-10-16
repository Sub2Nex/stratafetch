//! Browser downloads remain paused until a confirmed native transfer has real data.
use crate::{
    Command, browser_integration, check_reply, download_dir, ipc, native::Message, state_dir,
};
use anyhow::{Context, Result, ensure};
use reqwest::{
    StatusCode,
    header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, HeaderMap},
};
use serde_json::{Value, json};
use std::{
    path::Path,
    time::{Duration, Instant},
};
use strata_core::Status;

fn validate_headers(
    status: StatusCode,
    headers: &HeaderMap,
    size: Option<u64>,
    mime: Option<&str>,
) -> Result<()> {
    if status == StatusCode::RANGE_NOT_SATISFIABLE
        && size == Some(0)
        && headers
            .get(CONTENT_RANGE)
            .is_some_and(|value| value == "bytes */0")
    {
        return Ok(());
    }
    ensure!(
        matches!(status, StatusCode::OK | StatusCode::PARTIAL_CONTENT),
        "The anonymous download returned HTTP {status}; continue in the browser"
    );
    let actual_size = if status == StatusCode::PARTIAL_CONTENT {
        let range = headers
            .get(CONTENT_RANGE)
            .context("Missing download range")?
            .to_str()?;
        let (start, end, total) = strata_core::network::parse_range(range)?;
        ensure!((start, end) == (0, 0), "Unexpected download probe range");
        Some(total)
    } else {
        headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
    };
    if let (Some(expected), Some(actual)) = (size, actual_size) {
        ensure!(
            expected == actual,
            "The URL needs browser session data or its size changed; continue in the browser"
        );
    }
    let actual_mime = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    ensure!(
        actual_mime != "text/html",
        "This URL returned a webpage; continue in the browser for login-dependent downloads"
    );
    if let Some(expected) = mime.map(|v| {
        v.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    }) {
        ensure!(
            expected.is_empty()
                || actual_mime.is_empty()
                || expected == "application/octet-stream"
                || actual_mime == expected,
            "The URL returned a different file type; continue in the browser"
        );
    }
    Ok(())
}

async fn preflight(url: &str, size: Option<u64>, mime: Option<&str>) -> Result<()> {
    let response = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(10))
        .user_agent(concat!("StrataFetch/", env!("CARGO_PKG_VERSION")))
        .build()?
        .get(url)
        .header("Accept-Encoding", "identity")
        .header("Range", "bytes=0-0")
        .send()
        .await
        .map_err(|error| anyhow::anyhow!(error.without_url()))?;
    validate_headers(response.status(), response.headers(), size, mime)
}

async fn wait_for_service(directory: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        if let Ok(Ok(reply)) = tokio::time::timeout(
            Duration::from_millis(500),
            ipc::call(directory, Command::List),
        )
        .await
            && reply.error.is_none()
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("StrataFetch did not become ready; continue in the browser")
}

async fn wait_for_data(directory: &Path, id: &str) -> Result<strata_core::Job> {
    let deadline = Instant::now() + Duration::from_secs(35);
    while Instant::now() < deadline {
        let reply = check_reply(ipc::call(directory, Command::List).await?)?;
        let job = reply
            .jobs
            .into_iter()
            .find(|job| job.id == id)
            .context("Native download was removed")?;
        match job.status {
            Status::Completed => return Ok(job),
            Status::Downloading if job.downloaded > 0 => return Ok(job),
            Status::Failed => anyhow::bail!(
                "{}",
                job.error.unwrap_or_else(|| "Native transfer failed".into())
            ),
            Status::Cancelled | Status::Paused => {
                anyhow::bail!("Native transfer stopped; continue in the browser")
            }
            _ => (),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("Native transfer did not receive data; continue in the browser")
}

pub fn capture(runtime: &tokio::runtime::Runtime, message: &Message) -> Result<Value> {
    let url = message.url.as_deref().context("Missing download URL")?;
    let identity = strata_core::metadata::identity(url)?;
    let name = message
        .name
        .as_deref()
        .map(strata_core::sanitize_filename)
        .unwrap_or(identity.filename);
    if !browser_integration::confirm_download(&name, &identity.host, &download_dir())? {
        return Ok(json!({"ok":false,"declined":true}));
    }
    runtime.block_on(transfer_confirmed(
        &state_dir(),
        &download_dir(),
        url,
        name,
        message.size,
        message.mime.as_deref(),
        || browser_integration::launch_desktop(false),
    ))
}

async fn transfer_confirmed(
    directory: &Path,
    destination: &Path,
    url: &str,
    name: String,
    size: Option<u64>,
    mime: Option<&str>,
    launch: impl FnOnce() -> Result<()>,
) -> Result<Value> {
    preflight(url, size, mime).await?;
    launch()?;
    wait_for_service(directory).await?;
    let reply = check_reply(
        ipc::call(
            directory,
            Command::Add {
                url: url.to_owned(),
                directory: destination.to_owned(),
                name: Some(name),
            },
        )
        .await?,
    )?;
    let id = reply
        .accepted_id
        .context("Manager did not acknowledge the download")?;
    let _ = browser_integration::request_attention(directory, Some(id.clone()), false);
    match wait_for_data(directory, &id).await {
        Ok(job) => Ok(
            json!({"ok":true,"download":{"id":job.id,"filename":job.destination.file_name().unwrap_or_default().to_string_lossy(),"status":job.status}}),
        ),
        Err(error) => {
            let _ = ipc::call(directory, Command::Cancel { id }).await;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn confirmed_handoff_acknowledges_real_bytes_and_preflight_failure_creates_no_job() {
        use axum::{Router, body::Body, routing::get};
        let app = Router::new().route(
            "/file",
            get(|| async {
                axum::http::Response::builder()
                    .header("Content-Type", "application/zip")
                    .body(Body::from("verified payload"))
                    .unwrap()
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("state");
        let destination = temp.path().join("out");
        let manager = strata_core::Manager::open(&directory, Default::default()).unwrap();
        let service = tokio::spawn({
            let directory = directory.clone();
            let manager = manager.clone();
            async move { ipc::serve(&directory, manager).await }
        });
        let bad = transfer_confirmed(
            &directory,
            &destination,
            &url,
            "file.zip".into(),
            Some(999),
            Some("application/zip"),
            || panic!("a rejected probe must not launch the app"),
        )
        .await;
        assert!(bad.is_err());
        assert!(manager.list().is_empty());
        let reply = transfer_confirmed(
            &directory,
            &destination,
            &url,
            "file.zip".into(),
            Some(16),
            Some("application/zip"),
            || Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(reply["ok"], true);
        let id = reply["download"]["id"].as_str().unwrap();
        let job = manager.get(id).unwrap();
        assert!(job.downloaded > 0 || job.status == Status::Completed);
        tokio::time::timeout(Duration::from_secs(5), async {
            while manager.is_running() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(std::fs::read(job.destination).unwrap(), b"verified payload");
        crate::shutdown(&manager).await;
        service.abort();
        server.abort();
    }
    #[test]
    fn login_responses_changed_files_and_inconsistent_ranges_stay_in_browser() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_RANGE, "bytes */0".parse().unwrap());
        assert!(
            validate_headers(StatusCode::RANGE_NOT_SATISFIABLE, &headers, Some(0), None).is_ok()
        );
        assert!(
            validate_headers(StatusCode::RANGE_NOT_SATISFIABLE, &headers, Some(1), None).is_err()
        );
        assert!(validate_headers(StatusCode::RANGE_NOT_SATISFIABLE, &headers, None, None).is_err());
        headers.insert(CONTENT_LENGTH, "100".parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/zip".parse().unwrap());
        assert!(
            validate_headers(StatusCode::OK, &headers, Some(100), Some("application/zip")).is_ok()
        );
        assert!(validate_headers(StatusCode::OK, &headers, Some(200), None).is_err());
        headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".parse().unwrap());
        assert!(validate_headers(StatusCode::OK, &headers, None, None).is_err());
        assert!(validate_headers(StatusCode::UNAUTHORIZED, &headers, None, None).is_err());
        headers.insert(CONTENT_TYPE, "application/zip".parse().unwrap());
        headers.insert(CONTENT_RANGE, "bytes 0-0/100".parse().unwrap());
        assert!(
            validate_headers(
                StatusCode::PARTIAL_CONTENT,
                &headers,
                Some(100),
                Some("application/zip")
            )
            .is_ok()
        );
        headers.insert(CONTENT_RANGE, "bytes 1-1/100".parse().unwrap());
        assert!(validate_headers(StatusCode::PARTIAL_CONTENT, &headers, Some(100), None).is_err());
    }
}
