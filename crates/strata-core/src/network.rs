//! Byte-accurate HTTP transfers.
use crate::{Job, Manager, Segment, Status, now, staging_path};
use anyhow::{Context, Result, bail, ensure};
use futures_util::StreamExt;
use reqwest::{Response, StatusCode, header::*};
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{Mutex, watch},
    task::JoinSet,
};
fn backoff(attempt: usize) -> Duration {
    Duration::from_millis(
        300 * (1u64 << attempt.min(6)) + uuid::Uuid::new_v4().as_bytes()[0] as u64,
    )
}

fn validator(response: &Response) -> Option<String> {
    response
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.starts_with('"') && s.ends_with('"'))
        .map(str::to_owned)
        .or_else(|| {
            response
                .headers()
                .get(LAST_MODIFIED)
                .and_then(|v| v.to_str().ok())
                .filter(|s| httpdate::parse_http_date(s).is_ok())
                .map(str::to_owned)
        })
}
/// Parses a satisfied Content-Range without accepting unknown or overflowing bounds.
pub fn parse_range(raw: &str) -> Result<(u64, u64, u64)> {
    let raw = raw
        .strip_prefix("bytes ")
        .context("Invalid Content-Range unit")?;
    let (range, total) = raw.split_once('/').context("Missing Content-Range total")?;
    let (start, end) = range
        .split_once('-')
        .context("Missing Content-Range bounds")?;
    let (start, end, total): (u64, u64, u64) = (start.parse()?, end.parse()?, total.parse()?);
    ensure!(start <= end && end < total, "Invalid Content-Range bounds");
    Ok((start, end, total))
}
fn check_range(
    response: &Response,
    start: u64,
    end: u64,
    total: u64,
    expected: &str,
) -> Result<()> {
    ensure!(
        response.status() == StatusCode::PARTIAL_CONTENT,
        "Server refused byte range (HTTP {}); restart required",
        response.status()
    );
    let raw = response
        .headers()
        .get(CONTENT_RANGE)
        .context("Missing Content-Range")?
        .to_str()?;
    ensure!(
        parse_range(raw)? == (start, end, total),
        "Content-Range mismatch; refusing inconsistent bytes"
    );
    ensure!(
        validator(response).as_deref() == Some(expected),
        "Resource validator changed; restart required"
    );
    if let Some(encoding) = response.headers().get(CONTENT_ENCODING) {
        ensure!(encoding == "identity", "Encoded range response is unsafe");
    }
    if let Some(length) = response.content_length() {
        ensure!(length == end - start + 1, "Range length mismatch");
    }
    Ok(())
}
async fn request(
    manager: &Manager,
    url: &str,
    range: Option<(u64, u64, &str)>,
) -> Result<Response> {
    let mut request = manager
        .0
        .client
        .get(url)
        .header(ACCEPT_ENCODING, "identity");
    if let Some((start, end, validator)) = range {
        request = request
            .header(RANGE, format!("bytes={start}-{end}"))
            .header(IF_RANGE, validator);
    }
    request
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e.without_url()))
}
fn retry_delay(response: &Response, attempt: usize) -> Result<Duration> {
    let wait = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.parse::<u64>().ok().or_else(|| {
                httpdate::parse_http_date(value).ok().map(|time| {
                    time.duration_since(std::time::SystemTime::now())
                        .unwrap_or_default()
                        .as_secs()
                })
            })
        })
        .unwrap_or(1 << attempt.min(6));
    ensure!(
        wait <= 3600,
        "Server requires a retry delay longer than one hour; retry later"
    );
    Ok(Duration::from_secs(wait))
}
struct Probe {
    range_total: Option<u64>,
    validator: Option<String>,
}

async fn probe(
    manager: &Manager,
    url: &str,
    cancel: &mut watch::Receiver<bool>,
) -> Result<Option<Probe>> {
    let mut attempts = 0;
    loop {
        let response = tokio::select! {
            response = manager.0.client.get(url).header(ACCEPT_ENCODING,"identity").header(RANGE,"bytes=0-0").send() => response.map_err(|error| anyhow::anyhow!(error.without_url())),
            _ = cancelled(cancel) => return Ok(None)
        };
        let delay = match response {
            Ok(response)
                if response.status().is_server_error()
                    || response.status() == StatusCode::TOO_MANY_REQUESTS =>
            {
                ensure!(
                    attempts < manager.0.options.retries,
                    "HTTP {}; probe retry limit reached",
                    response.status()
                );
                attempts += 1;
                retry_delay(&response, attempts)?
            }
            Ok(response) => {
                let valid = validator(&response);
                if response.status() == StatusCode::PARTIAL_CONTENT {
                    let raw = response
                        .headers()
                        .get(CONTENT_RANGE)
                        .context("Missing Content-Range in probe")?
                        .to_str()?;
                    let (start, end, total) = parse_range(raw)?;
                    ensure!((start, end) == (0, 0), "Invalid probe range");
                    if let Some(value) = &valid {
                        check_range(&response, 0, 0, total, value)?;
                    }
                    let body = consume_probe(response, cancel).await;
                    match body {
                        Ok(Some(())) => {
                            return Ok(Some(Probe {
                                range_total: Some(total),
                                validator: valid,
                            }));
                        }
                        Ok(None) => return Ok(None),
                        Err(error) => {
                            if attempts >= manager.0.options.retries {
                                return Err(error);
                            }
                            attempts += 1;
                            backoff(attempts)
                        }
                    }
                } else {
                    ensure!(
                        response.status().is_success()
                            || (response.status() == StatusCode::RANGE_NOT_SATISFIABLE
                                && response
                                    .headers()
                                    .get(CONTENT_RANGE)
                                    .is_some_and(|v| v == "bytes */0")),
                        "HTTP {} during probe",
                        response.status()
                    );
                    return Ok(Some(Probe {
                        range_total: None,
                        validator: valid,
                    }));
                }
            }
            Err(error) => {
                if attempts >= manager.0.options.retries {
                    return Err(error);
                }
                attempts += 1;
                backoff(attempts)
            }
        };
        tokio::select! { _ = tokio::time::sleep(delay) => (), _ = cancelled(cancel) => return Ok(None) }
    }
}

async fn consume_probe(
    response: Response,
    cancel: &mut watch::Receiver<bool>,
) -> Result<Option<()>> {
    let mut body = response.bytes_stream();
    let mut received = 0;
    loop {
        let chunk = tokio::select! {
            chunk = body.next() => chunk,
            _ = cancelled(cancel) => return Ok(None)
        };
        let Some(bytes) = chunk else { break };
        received += bytes.map_err(|e| anyhow::anyhow!(e.without_url()))?.len();
        ensure!(received <= 1, "Oversized probe response");
    }
    ensure!(received == 1, "Truncated probe response");
    Ok(Some(()))
}
async fn cancelled(rx: &mut watch::Receiver<bool>) {
    if *rx.borrow() {
        return;
    }
    let _ = rx.changed().await;
}

fn safe_staging_metadata(metadata: &std::fs::Metadata) -> Result<()> {
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "Unsafe staging file"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "Staging reparse points are not supported"
        );
    }
    Ok(())
}

fn check_destination_parent(job: &Job) -> Result<()> {
    let parent = job
        .destination
        .parent()
        .context("Missing destination directory")?;
    ensure!(
        parent.canonicalize()? == parent,
        "Destination directory was redirected; restart in a safe folder"
    );
    Ok(())
}

pub(crate) async fn run(
    manager: &Manager,
    id: &str,
    mut cancel: watch::Receiver<bool>,
) -> Result<()> {
    let mut job = manager.get(id)?;
    if *cancel.borrow() {
        return Ok(());
    }
    {
        let _controls = manager.0.controls.lock().unwrap();
        if *cancel.borrow() {
            return Ok(());
        }
        job.status = Status::Downloading;
        manager.update(job.clone())?;
    }
    manager
        .0
        .meter
        .lock()
        .unwrap()
        .insert(id.to_owned(), (Instant::now(), job.downloaded));
    let stage = staging_path(&job);
    check_destination_parent(&job)?;
    if let Ok(meta) = tokio::fs::symlink_metadata(&stage).await {
        safe_staging_metadata(&meta)?;
    }
    // Probe the actual GET path; HEAD and Accept-Ranges alone are not reliable.
    let Some(probe) = probe(manager, &job.url, &mut cancel).await? else {
        return Ok(());
    };
    let valid = probe.validator;
    let range_total = probe.range_total;
    let segmented = range_total.is_some() && valid.is_some();
    job.resumable = segmented;
    if job.downloaded > 0 {
        ensure!(
            segmented && job.validator == valid && job.size == range_total,
            "Remote version cannot be verified; explicitly restart this download"
        );
        let meta = tokio::fs::metadata(&stage)
            .await
            .context("Partial file missing; restart required")?;
        ensure!(
            meta.len() == job.size.unwrap_or(0),
            "Partial file size changed; restart required"
        );
        for segment in &job.segments {
            if segment.done > 0 {
                let digest = tokio::select! { digest = hash_prefix(&stage, segment.start, segment.done) => digest?, _ = cancelled(&mut cancel) => return Ok(()) };
                ensure!(
                    Some(format!("{:x}", digest.finalize())) == segment.sha256,
                    "Partial data checksum changed; restart required"
                );
            }
        }
    } else {
        job.validator = valid;
        job.size = range_total;
        job.segments = if segmented {
            let total = range_total.unwrap();
            let count = manager
                .settings()
                .connections
                .clamp(1, 16)
                .min(total.div_ceil(256 * 1024) as usize)
                .max(1);
            let chunk = total.div_ceil(count as u64);
            (0..count)
                .filter_map(|i| {
                    let start = i as u64 * chunk;
                    (start < total).then_some(Segment {
                        start,
                        end: start.saturating_add(chunk - 1).min(total - 1),
                        done: 0,
                        sha256: None,
                    })
                })
                .collect()
        } else {
            vec![Segment {
                start: 0,
                end: 0,
                done: 0,
                sha256: None,
            }]
        };
    }
    job.connections = if segmented {
        job.segments
            .iter()
            .filter(|s| s.done < s.end - s.start + 1)
            .count()
    } else {
        1
    };
    {
        let _controls = manager.0.controls.lock().unwrap();
        if *cancel.borrow() {
            return Ok(());
        }
        manager.update(job.clone())?;
    }
    let mut file_options = OpenOptions::new();
    file_options.read(true).write(true);
    if stage.exists() {
        file_options.truncate(job.downloaded == 0);
    } else {
        file_options.create_new(true);
    }
    let file = file_options.open(&stage).await?;
    if segmented {
        file.set_len(job.size.unwrap()).await?;
    }
    drop(file);
    let shared = Arc::new(Mutex::new(job.clone()));
    let mut workers = JoinSet::new();
    for index in 0..job.segments.len() {
        let manager = manager.clone();
        let shared = shared.clone();
        let rx = cancel.clone();
        workers.spawn(async move { transfer(&manager, shared, index, segmented, rx).await });
    }
    let mut failure = None;
    while let Some(result) = workers.join_next().await {
        match result {
            Ok(Ok(())) => (),
            Ok(Err(error)) => {
                failure = Some(error);
                workers.abort_all();
            }
            Err(error) if error.is_cancelled() => (),
            Err(error) => {
                failure = Some(error.into());
                workers.abort_all();
            }
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    if *cancel.borrow() {
        return Ok(());
    }
    let mut finished = shared.lock().await.clone();
    ensure!(
        finished.size.is_none_or(|size| size == finished.downloaded),
        "Downloaded size mismatch"
    );
    check_destination_parent(&finished)?;
    let metadata = tokio::fs::symlink_metadata(&stage).await?;
    safe_staging_metadata(&metadata)?;
    ensure!(
        metadata.len() == finished.downloaded,
        "Staging file size changed before completion"
    );
    // Compare with hashes accumulated from HTTP bytes, not a fresh checksum alone.
    // A fresh checksum would otherwise bless local damage as the completed file.
    for segment in &finished.segments {
        let digest = tokio::select! { digest = hash_prefix(&stage, segment.start, segment.done) => digest?, _ = cancelled(&mut cancel) => return Ok(()) };
        ensure!(
            Some(format!("{:x}", digest.finalize())) == segment.sha256,
            "Staging data checksum changed before completion"
        );
    }
    let mut file = tokio::fs::File::open(&stage).await?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = tokio::select! { n = file.read(&mut buffer) => n?, _=cancelled(&mut cancel)=>return Ok(()) };
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    drop(file);
    // Hard-link publication is atomic and fails if the destination already exists.
    // Keep control lock so a pause cannot race publication/state transition.
    let _controls = manager.0.controls.lock().unwrap();
    if *cancel.borrow() {
        return Ok(());
    }
    std::fs::hard_link(&stage,&finished.destination).context("Cannot finalize without overwriting; destination may exist or filesystem may not support hard links")?;
    finished.sha256 = Some(format!("{:x}", hash.finalize()));
    finished.status = Status::Completed;
    finished.completed = Some(now());
    finished.size = Some(finished.downloaded);
    finished.speed = 0.0;
    finished.connections = 0;
    manager.update(finished)?;
    if let Err(error) = std::fs::remove_file(stage) {
        eprintln!("Completed staging cleanup failed: {error}");
    }
    Ok(())
}

async fn transfer(
    manager: &Manager,
    shared: Arc<Mutex<Job>>,
    index: usize,
    ranged: bool,
    mut cancel: watch::Receiver<bool>,
) -> Result<()> {
    let start_time = Instant::now();
    let mut checkpoint = Instant::now();
    let mut pending = 0u64;
    let initial = shared.lock().await.clone();
    let segment = &initial.segments[index];
    let stage = staging_path(&initial);
    let mut digest = tokio::select! { digest = hash_prefix(&stage, segment.start, segment.done) => digest?, _ = cancelled(&mut cancel) => return Ok(()) };
    if ranged && segment.done == segment.end - segment.start + 1 {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .open(staging_path(&initial))
        .await?;
    let mut attempts = 0;
    loop {
        if *cancel.borrow() {
            return Ok(());
        }
        let job = shared.lock().await.clone();
        let seg = &job.segments[index];
        let offset = seg.start + seg.done;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let response = tokio::select! { r=request(manager,&job.url,if ranged {Some((offset,seg.end,job.validator.as_deref().unwrap()))} else {None})=>r, _=cancelled(&mut cancel)=>return Ok(()) };
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                if attempts >= manager.0.options.retries {
                    return Err(e);
                }
                attempts += 1;
                tokio::select! {_=tokio::time::sleep(backoff(attempts))=>(),_=cancelled(&mut cancel)=>return Ok(())};
                continue;
            }
        };
        if response.status().is_server_error() || response.status() == StatusCode::TOO_MANY_REQUESTS
        {
            ensure!(
                attempts < manager.0.options.retries,
                "HTTP {}; retry limit reached",
                response.status()
            );
            attempts += 1;
            let delay = retry_delay(&response, attempts)?;
            tokio::select! {_=tokio::time::sleep(delay)=>(),_=cancelled(&mut cancel)=>return Ok(())};
            continue;
        }
        if ranged {
            check_range(
                &response,
                offset,
                seg.end,
                job.size.unwrap(),
                job.validator.as_deref().unwrap(),
            )?;
        } else {
            ensure!(
                response.status() == StatusCode::OK,
                "Unexpected HTTP {}",
                response.status()
            );
            if let Some(encoding) = response.headers().get(CONTENT_ENCODING) {
                ensure!(encoding == "identity", "Server ignored identity encoding");
            }
            let mut j = shared.lock().await;
            j.size = response.content_length();
        }
        let expected = if ranged {
            Some(seg.end - offset + 1)
        } else {
            response.content_length()
        };
        let mut received = 0u64;
        let mut stream = response.bytes_stream();
        let mut stream_error = None;
        loop {
            let chunk = tokio::select! { chunk=stream.next()=>chunk, _=cancelled(&mut cancel)=> {file.sync_data().await?; checkpoint_job(manager,&shared,index,pending,&digest,start_time).await?; return Ok(());} };
            let Some(chunk) = chunk else { break };
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => {
                    stream_error = Some(anyhow::anyhow!(e.without_url()));
                    break;
                }
            };
            received += bytes.len() as u64;
            ensure!(
                expected.is_none_or(|n| received <= n),
                "Server sent too many bytes"
            );
            // Pace bounded writes before committing them. A whole HTTP chunk can
            // represent minutes at a low limit and must not hide checkpoints.
            let per_connection = job.bytes_per_second as f64 / job.connections.max(1) as f64;
            let global_rate = manager.settings().global_bytes_per_second as f64;
            let pace = match (per_connection > 0.0, global_rate > 0.0) {
                (true, true) => per_connection.min(global_rate),
                (true, false) => per_connection,
                (false, true) => global_rate,
                (false, false) => 0.0,
            };
            let quantum = if pace == 0.0 {
                16 * 1024
            } else {
                (pace / 20.0).ceil().clamp(1.0, 16_384.0) as usize
            };
            for part in bytes.chunks(quantum) {
                tokio::select! {
                    _ = async {
                        global_throttle(manager, part.len()).await;
                        if per_connection > 0.0 {
                            tokio::time::sleep(Duration::from_secs_f64(part.len() as f64 / per_connection)).await;
                        }
                    } => (),
                    _ = cancelled(&mut cancel) => {
                        file.sync_data().await?;
                        checkpoint_job(manager,&shared,index,pending,&digest,start_time).await?;
                        return Ok(());
                    }
                }
                write_chunk(&mut file, part, &mut digest, &mut pending).await?;
                if checkpoint.elapsed() >= Duration::from_millis(250) {
                    file.sync_data().await?;
                    checkpoint_job(manager, &shared, index, pending, &digest, start_time).await?;
                    pending = 0;
                    checkpoint = Instant::now();
                }
            }
        }
        file.sync_data().await?;
        checkpoint_job(manager, &shared, index, pending, &digest, start_time).await?;
        pending = 0;
        if stream_error.is_none() && expected.is_none_or(|n| received == n) {
            return Ok(());
        }
        if !ranged {
            bail!("Single-stream response interrupted; restart required");
        }
        ensure!(
            attempts < manager.0.options.retries,
            "Range response truncated; retry limit reached"
        );
        attempts += 1;
        tokio::select! {_=tokio::time::sleep(backoff(attempts))=>(),_=cancelled(&mut cancel)=>return Ok(())};
    }
}
/// Serial payload pacing across all jobs, with bounded quanta and no initial burst.
async fn global_throttle(manager: &Manager, bytes: usize) {
    if manager.settings().global_bytes_per_second == 0 {
        return;
    }
    let _gate = manager.0.bandwidth.lock().await;
    let mut remaining = bytes as f64;
    let mut last = Instant::now();
    loop {
        let rate = manager.settings().global_bytes_per_second;
        if rate == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_secs_f64(
            (remaining / rate as f64).min(0.020),
        ))
        .await;
        remaining -= last.elapsed().as_secs_f64() * rate as f64;
        last = Instant::now();
        if remaining <= 0.0 {
            return;
        }
    }
}
async fn checkpoint_job(
    manager: &Manager,
    shared: &Mutex<Job>,
    index: usize,
    bytes: u64,
    digest: &Sha256,
    _started: Instant,
) -> Result<()> {
    let mut job = shared.lock().await;
    let _controls = manager.0.controls.lock().unwrap();
    job.segments[index].done += bytes;
    job.downloaded = job.segments.iter().map(|s| s.done).sum();
    job.segments[index].sha256 = Some(format!("{:x}", digest.clone().finalize()));
    {
        let mut meters = manager.0.meter.lock().unwrap();
        let meter = meters
            .entry(job.id.clone())
            .or_insert((Instant::now(), job.downloaded));
        let elapsed = meter.0.elapsed().as_secs_f64();
        if elapsed >= 0.2 {
            job.speed = job.downloaded.saturating_sub(meter.1) as f64 / elapsed;
            *meter = (Instant::now(), job.downloaded);
        }
    }
    let state = manager.get(&job.id)?.status;
    job.status = state;
    manager.update(job.clone())
}
async fn hash_prefix(path: &std::path::Path, start: u64, length: u64) -> Result<Sha256> {
    let mut digest = Sha256::new();
    if length == 0 {
        return Ok(digest);
    }
    let mut file = tokio::fs::File::open(path).await?;
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let mut left = length;
    let mut buffer = vec![0; 64 * 1024];
    while left > 0 {
        let limit = left.min(buffer.len() as u64) as usize;
        let n = file.read(&mut buffer[..limit]).await?;
        ensure!(n > 0, "Partial file truncated");
        digest.update(&buffer[..n]);
        left -= n as u64;
    }
    Ok(digest)
}

// Only fully written bytes may enter a later durable checkpoint. A partial OS
// write followed by disk-full/interruption leaves the old checkpoint authoritative.
async fn write_chunk(
    writer: &mut (impl tokio::io::AsyncWrite + Unpin),
    bytes: &[u8],
    digest: &mut Sha256,
    pending: &mut u64,
) -> std::io::Result<()> {
    writer.write_all(bytes).await?;
    digest.update(bytes);
    *pending += bytes.len() as u64;
    Ok(())
}

#[cfg(test)]
mod write_fault_tests {
    use super::*;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };
    struct FailingDisk {
        remaining: usize,
        bytes: Vec<u8>,
        error: i32,
    }
    impl tokio::io::AsyncWrite for FailingDisk {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            if self.remaining == 0 {
                return Poll::Ready(Err(std::io::Error::from_raw_os_error(self.error)));
            }
            let n = bytes.len().min(self.remaining);
            self.bytes.extend_from_slice(&bytes[..n]);
            self.remaining -= n;
            Poll::Ready(Ok(n))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
    #[tokio::test]
    async fn partial_disk_full_and_interrupted_writes_do_not_advance_checkpoints() {
        // Windows ERROR_DISK_FULL / ERROR_OPERATION_ABORTED; Unix ENOSPC / EIO.
        #[cfg(windows)]
        let errors = [112, 995];
        #[cfg(not(windows))]
        let errors = [28, 5];
        for error in errors {
            let mut disk = FailingDisk {
                remaining: 3,
                bytes: vec![],
                error,
            };
            let mut digest = Sha256::new();
            digest.update(b"durable prefix");
            let expected = digest.clone().finalize();
            let mut pending = 14;
            assert_eq!(
                write_chunk(&mut disk, b"abcdef", &mut digest, &mut pending)
                    .await
                    .unwrap_err()
                    .raw_os_error(),
                Some(error)
            );
            assert_eq!(disk.bytes, b"abc");
            assert_eq!(pending, 14);
            assert_eq!(digest.finalize(), expected);
        }
    }
}
