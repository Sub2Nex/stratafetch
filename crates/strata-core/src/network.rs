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
async fn cancelled(rx: &mut watch::Receiver<bool>) {
    if *rx.borrow() {
        return;
    }
    let _ = rx.changed().await;
}

pub(crate) async fn run(
    manager: &Manager,
    id: &str,
    mut cancel: watch::Receiver<bool>,
) -> Result<()> {
    loop {
        if *cancel.borrow() {
            return Ok(());
        }
        {
            let _controls = manager.0.controls.lock().unwrap();
            let jobs = manager.list();
            if !*cancel.borrow()
                && jobs
                    .iter()
                    .filter(|j| j.status == Status::Downloading)
                    .count()
                    < manager.settings().concurrent_jobs
                && jobs
                    .iter()
                    .find(|j| j.status == Status::Queued)
                    .is_some_and(|j| j.id == id)
            {
                let mut job = manager.get(id)?;
                job.status = Status::Downloading;
                manager.update(job)?;
                break;
            }
        }
        tokio::select! {_=tokio::time::sleep(Duration::from_millis(25))=>(),_=cancelled(&mut cancel)=>return Ok(())};
    }
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
    if let Ok(meta) = tokio::fs::symlink_metadata(&stage).await {
        ensure!(
            meta.is_file() && !meta.file_type().is_symlink(),
            "Unsafe staging file"
        );
    }
    // Probe the actual GET path; HEAD and Accept-Ranges alone are not reliable.
    let probe = tokio::select! {
        response = manager.0.client.get(&job.url).header(ACCEPT_ENCODING,"identity").header(RANGE,"bytes=0-0").send() => response.map_err(|e| anyhow::anyhow!(e.without_url()))?,
        _ = cancelled(&mut cancel) => return Ok(())
    };
    let valid = validator(&probe);
    let range_total = if probe.status() == StatusCode::PARTIAL_CONTENT {
        let raw = probe
            .headers()
            .get(CONTENT_RANGE)
            .context("Missing Content-Range in probe")?
            .to_str()?;
        let (start, end, total) = parse_range(raw)?;
        ensure!((start, end) == (0, 0), "Invalid probe range");
        if let Some(v) = &valid {
            check_range(&probe, 0, 0, total, v)?;
        }
        let mut body = probe.bytes_stream();
        let mut received = 0;
        loop {
            let chunk =
                tokio::select! {chunk=body.next()=>chunk,_=cancelled(&mut cancel)=>return Ok(())};
            let Some(bytes) = chunk else {
                break;
            };
            received += bytes.map_err(|e| anyhow::anyhow!(e.without_url()))?.len();
            ensure!(received <= 1, "Oversized probe response");
        }
        ensure!(received == 1, "Truncated probe response");
        Some(total)
    } else if probe.status() == StatusCode::RANGE_NOT_SATISFIABLE
        && probe
            .headers()
            .get(CONTENT_RANGE)
            .is_some_and(|v| v == "bytes */0")
    {
        None
    } else {
        ensure!(
            probe.status().is_success(),
            "HTTP {} during probe",
            probe.status()
        );
        None
    };
    let segmented = range_total.is_some() && valid.is_some();
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
                let digest = hash_prefix(&stage, segment.start, segment.done).await?;
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
    let mut digest = hash_prefix(&staging_path(&initial), segment.start, segment.done).await?;
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
            let wait = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|h| {
                    h.parse::<u64>().ok().or_else(|| {
                        httpdate::parse_http_date(h).ok().map(|time| {
                            time.duration_since(std::time::SystemTime::now())
                                .unwrap_or_default()
                                .as_secs()
                        })
                    })
                })
                .unwrap_or(1 << attempts.min(6));
            ensure!(
                wait <= 3600,
                "Server requires a retry delay longer than one hour; retry later"
            );
            tokio::select! {_=tokio::time::sleep(Duration::from_secs(wait))=>(),_=cancelled(&mut cancel)=>return Ok(())};
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
            for part in bytes.chunks(16 * 1024) {
                tokio::select! {
                    _ = global_throttle(manager, part.len()) => (),
                    _ = cancelled(&mut cancel) => {
                        file.sync_data().await?;
                        checkpoint_job(manager,&shared,index,pending,&digest,start_time).await?;
                        return Ok(());
                    }
                }
                file.write_all(part).await?;
                digest.update(part);
                pending += part.len() as u64;
            }
            if job.bytes_per_second > 0 {
                let per_connection = job.bytes_per_second as f64 / job.connections.max(1) as f64;
                tokio::select! {_=tokio::time::sleep(Duration::from_secs_f64(bytes.len() as f64/per_connection))=>(),_=cancelled(&mut cancel)=>{file.sync_data().await?;checkpoint_job(manager,&shared,index,pending,&digest,start_time).await?;return Ok(());}}
            }
            if checkpoint.elapsed() >= Duration::from_millis(250) {
                file.sync_data().await?;
                checkpoint_job(manager, &shared, index, pending, &digest, start_time).await?;
                pending = 0;
                checkpoint = Instant::now();
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
        tokio::time::sleep(Duration::from_millis(20)).await;
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
