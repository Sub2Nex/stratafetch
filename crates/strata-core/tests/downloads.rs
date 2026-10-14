use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Response, StatusCode},
    routing::get,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use strata_core::{Manager, Options, Status};
use tempfile::TempDir;

#[tokio::test]
async fn metadata_probe_uses_headers_and_handles_head_fallback_without_creating_jobs() {
    use axum::http::Method;
    use axum::routing::any;
    let gets = Arc::new(AtomicUsize::new(0));
    let observed = gets.clone();
    let app = Router::new()
        .route(
            "/headers",
            any(move |method: Method| {
                let observed = observed.clone();
                async move {
                    if method == Method::GET {
                        observed.fetch_add(1, Ordering::SeqCst);
                    }
                    Response::builder()
                        .header("content-length", "12345")
                        .header(
                            "content-disposition",
                            "attachment; filename=fallback.zip; filename*=UTF-8'en'my%20archive.zip",
                        )
                        .body(Body::empty())
                        .unwrap()
                }
            }),
        )
        .route(
            "/fallback",
            any(|method: Method, headers: HeaderMap| async move {
                if method == Method::HEAD {
                    return Response::builder().status(405).body(Body::empty()).unwrap();
                }
                assert_eq!(headers.get("range").unwrap(), "bytes=0-0");
                Response::builder()
                    .status(206)
                    .header("content-range", "bytes 0-0/98765")
                    .header("content-length", "1")
                    .body(Body::from("x"))
                    .unwrap()
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(dir.path(), Default::default()).unwrap();
    let metadata = strata_core::metadata::probe(&format!("http://{address}/headers"))
        .await
        .unwrap();
    assert_eq!(metadata.size, Some(12345));
    assert_eq!(metadata.filename, "my archive.zip");
    assert_eq!(gets.load(Ordering::SeqCst), 0);
    let fallback = strata_core::metadata::probe(&format!("http://{address}/fallback"))
        .await
        .unwrap();
    assert_eq!(fallback.size, Some(98765));
    assert!(manager.list().is_empty());
    task.abort();
}

#[tokio::test]
async fn aggregate_bandwidth_is_shared_and_settings_survive_reopen() {
    let (url, state, task) = server(0, 128 * 1024).await;
    let dir = TempDir::new().unwrap();
    let state_dir = dir.path().join("state");
    let manager = Manager::open(&state_dir, Options::default()).unwrap();
    let settings = strata_core::Settings {
        connections: 2,
        concurrent_jobs: 2,
        global_bytes_per_second: 256 * 1024,
        ..Default::default()
    };
    manager.set_settings(settings.clone()).unwrap();
    let a = manager.add(&url, dir.path(), Some("a.bin")).unwrap();
    let b = manager.add(&url, dir.path(), Some("b.bin")).unwrap();
    let started = std::time::Instant::now();
    manager.start(&a.id).unwrap();
    manager.start(&b.id).unwrap();
    let a = wait(&manager, &a.id).await;
    let b = manager.get(&b.id).unwrap();
    assert_eq!(a.status, Status::Completed, "{:?}", a.error);
    assert_eq!(b.status, Status::Completed, "{:?}", b.error);
    assert!(started.elapsed() >= std::time::Duration::from_millis(950));
    assert_eq!(std::fs::read(a.destination).unwrap(), *state.data);
    assert_eq!(std::fs::read(b.destination).unwrap(), *state.data);
    drop(manager);
    assert_eq!(
        Manager::open(&state_dir, Options::default())
            .unwrap()
            .settings(),
        settings
    );
    task.abort();
}

#[tokio::test]
async fn changing_limits_unblocks_queue_and_cancels_slow_throttle() {
    let (url, _, task) = server(0, 512 * 1024).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    manager
        .set_settings(strata_core::Settings {
            connections: 2,
            concurrent_jobs: 1,
            global_bytes_per_second: 1,
            ..Default::default()
        })
        .unwrap();
    let a = manager.add(&url, dir.path(), Some("a.bin")).unwrap();
    let b = manager.add(&url, dir.path(), Some("b.bin")).unwrap();
    manager.start(&a.id).unwrap();
    manager.start(&b.id).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    assert_eq!(manager.get(&a.id).unwrap().status, Status::Downloading);
    assert_eq!(manager.get(&b.id).unwrap().status, Status::Queued);
    manager.pause(&a.id).unwrap();
    manager
        .set_settings(strata_core::Settings {
            connections: 2,
            concurrent_jobs: 2,
            global_bytes_per_second: 0,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(wait(&manager, &b.id).await.status, Status::Completed);
    assert_eq!(manager.get(&a.id).unwrap().status, Status::Paused);
    manager.start(&a.id).unwrap();
    assert_eq!(wait(&manager, &a.id).await.status, Status::Completed);
    assert_eq!(manager.get(&b.id).unwrap().segments.len(), 2);
    task.abort();
}

#[test]
fn concurrent_adds_reserve_distinct_destinations_and_sequence() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..16 {
            scope.spawn(|| {
                manager
                    .add("https://example.com/file", dir.path(), Some("same.bin"))
                    .unwrap();
            });
        }
    });
    let jobs = manager.list();
    assert_eq!(jobs.len(), 16);
    let destinations: std::collections::HashSet<_> =
        jobs.iter().map(|job| &job.destination).collect();
    let sequences: std::collections::HashSet<_> = jobs.iter().map(|job| job.sequence).collect();
    assert_eq!(destinations.len(), 16);
    assert_eq!(sequences.len(), 16);
}

#[tokio::test]
async fn organization_schedule_and_history_recipes_are_safe() {
    let (url, state, task) = server(0, 1024).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    manager
        .set_settings(strata_core::Settings {
            organize_by_extension: true,
            ..Default::default()
        })
        .unwrap();
    let scheduled = manager.add(&url, dir.path(), Some("report.PDF")).unwrap();
    assert_eq!(scheduled.category, "Documents");
    assert_eq!(
        scheduled.destination.parent().unwrap(),
        dir.path().join("Documents").canonicalize().unwrap()
    );
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    manager
        .schedule(&scheduled.id, "Work", Some(future))
        .unwrap();
    manager.configure(&scheduled.id, 100, 0).unwrap();
    manager.start(&scheduled.id).unwrap();
    let immediate = manager.add(&url, dir.path(), Some("now.zip")).unwrap();
    manager.start(&immediate.id).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if manager.get(&immediate.id).unwrap().status == Status::Completed {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(manager.get(&scheduled.id).unwrap().status, Status::Queued);
    assert_eq!(std::fs::read(immediate.destination).unwrap(), *state.data);
    manager.pause(&scheduled.id).unwrap();
    while manager.is_running() {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let export = dir.path().join("history.json");
    manager.export_history(&export).unwrap();
    assert!(manager.export_history(&export).is_err());
    let imported = Manager::open(&dir.path().join("import-state"), Options::default()).unwrap();
    imported
        .import_history(&export, &dir.path().join("imported"))
        .unwrap();
    assert_eq!(imported.list().len(), 2);
    for job in imported.list() {
        assert_eq!(job.status, Status::Paused);
        assert_eq!(job.downloaded, 0);
        assert!(job.segments.is_empty());
        assert!(job.sha256.is_none());
        assert_ne!(job.id, scheduled.id);
    }
    let malformed = dir.path().join("bad.json");
    std::fs::write(&malformed,r#"{"version":1,"entries":[{"url":"file:///secret","filename":"../bad","category":"Other","priority":0,"bytes_per_second":0}]}"#).unwrap();
    assert!(imported.import_history(&malformed, dir.path()).is_err());
    assert_eq!(imported.list().len(), 2);
    task.abort();
}

#[tokio::test]
async fn repeated_pause_resume_preserves_all_segments() {
    let (url, state, task) = server(0, 2 * 1024 * 1024).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), Some("repeated.bin")).unwrap();
    manager.configure(&job.id, 0, 512 * 1024).unwrap();
    let mut checkpoint = 0;
    for _ in 0..5 {
        manager.start(&job.id).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        manager.pause(&job.id).unwrap();
        let paused = wait(&manager, &job.id).await;
        assert_eq!(paused.status, Status::Paused);
        assert!(paused.downloaded >= checkpoint);
        checkpoint = paused.downloaded;
    }
    assert!(checkpoint > 0 && checkpoint < state.data.len() as u64);
    manager.configure(&job.id, 0, 0).unwrap();
    manager.start(&job.id).unwrap();
    let completed = wait(&manager, &job.id).await;
    assert_eq!(completed.status, Status::Completed, "{:?}", completed.error);
    assert_eq!(std::fs::read(completed.destination).unwrap(), *state.data);
    task.abort();
}

#[derive(Clone)]
struct Server {
    data: Arc<Vec<u8>>,
    mode: u8,
    version: Arc<AtomicUsize>,
    ranges: Arc<AtomicUsize>,
    failures: Arc<AtomicUsize>,
}
async fn handler(State(server): State<Server>, headers: HeaderMap) -> Response<Body> {
    let etag = format!("\"v{}\"", server.version.load(Ordering::SeqCst));
    if server.mode == 6 {
        return Response::builder()
            .status(302)
            .header("Location", "/file")
            .body(Body::empty())
            .unwrap();
    }
    if let Some(range) = headers.get("range").and_then(|v| v.to_str().ok())
        && server.mode != 1
        && server.mode != 5
    {
        let (a, b) = range
            .strip_prefix("bytes=")
            .unwrap()
            .split_once('-')
            .unwrap();
        let a: usize = a.parse().unwrap();
        let b: usize = b.parse().unwrap();
        server.ranges.fetch_add(1, Ordering::SeqCst);
        if a >= server.data.len() {
            return Response::builder()
                .status(416)
                .header("Content-Range", format!("bytes */{}", server.data.len()))
                .body(Body::empty())
                .unwrap();
        }
        let b = b.min(server.data.len() - 1);
        if server.mode == 7 && b > 0 && server.failures.fetch_add(1, Ordering::SeqCst) == 0 {
            let chunks = vec![
                Ok(server.data[a..=a + 1023].to_vec()),
                Err(std::io::Error::other("fixture disconnect")),
            ];
            return Response::builder()
                .status(206)
                .header("ETag", &etag)
                .header(
                    "Content-Range",
                    format!("bytes {a}-{b}/{}", server.data.len()),
                )
                .header("Content-Length", (b - a + 1).to_string())
                .body(Body::from_stream(futures_util::stream::iter(chunks)))
                .unwrap();
        }
        if server.mode == 11 && b > 0 && server.failures.fetch_add(1, Ordering::SeqCst) == 0 {
            return Response::builder()
                .status(429)
                .header("Retry-After", "0")
                .body(Body::empty())
                .unwrap();
        }
        let advertised = if server.mode == 2 { a + 1 } else { a };
        let body = if server.mode == 3 && b > 0 {
            server.data[a..b].to_vec()
        } else {
            server.data[a..=b].to_vec()
        };
        if server.mode == 4 && b > 0 {
            return Response::builder()
                .status(200)
                .header("ETag", &etag)
                .body(Body::from(body))
                .unwrap();
        }
        let mut response = Response::builder().status(206).header(
            "Content-Range",
            format!(
                "bytes {advertised}-{b}/{}",
                if server.mode == 13 {
                    u64::MAX
                } else {
                    server.data.len() as u64
                }
            ),
        );
        if server.mode == 9 {
            response = response.header(
                "Last-Modified",
                if server.version.load(Ordering::SeqCst) == 1 {
                    "Sun, 11 Oct 2026 07:00:00 GMT"
                } else {
                    "Sun, 11 Oct 2026 08:00:00 GMT"
                },
            );
        } else if server.mode != 8 {
            response = response.header("ETag", &etag);
        }
        return response.body(Body::from(body)).unwrap();
    }
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("ETag", etag);
    if server.mode == 5 {
        response = response.header("Transfer-Encoding", "chunked");
        let chunks = server
            .data
            .chunks(8192)
            .map(|c| Ok::<_, std::io::Error>(c.to_vec()))
            .collect::<Vec<_>>();
        response
            .body(Body::from_stream(futures_util::stream::iter(chunks)))
            .unwrap()
    } else {
        response
            .body(Body::from(server.data.as_ref().clone()))
            .unwrap()
    }
}
async fn server(mode: u8, size: usize) -> (String, Server, tokio::task::JoinHandle<()>) {
    let state = Server {
        data: Arc::new((0..size).map(|i| (i % 251) as u8).collect()),
        mode,
        version: Arc::new(AtomicUsize::new(1)),
        ranges: Arc::new(AtomicUsize::new(0)),
        failures: Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/file", get(handler))
        .route(
            "/redirect",
            get(|| async {
                Response::builder()
                    .status(302)
                    .header("Location", "/file")
                    .body(Body::empty())
                    .unwrap()
            }),
        )
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}/file"), state, task)
}
async fn wait(manager: &Manager, id: &str) -> strata_core::Job {
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let job = manager.get(id).unwrap();
            if !matches!(job.status, Status::Queued | Status::Downloading) && !manager.is_running()
            {
                return job;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn downloads_and_safe_fallbacks() {
    for mode in [0, 1, 5] {
        let (url, state, task) = server(mode, 1_100_000).await;
        let dir = TempDir::new().unwrap();
        let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
        let job = manager
            .add(&url, &dir.path().join("out"), Some("test.bin"))
            .unwrap();
        manager.start(&job.id).unwrap();
        let job = wait(&manager, &job.id).await;
        assert_eq!(job.status, Status::Completed, "{:?}", job.error);
        assert_eq!(std::fs::read(&job.destination).unwrap(), *state.data);
        assert!(job.sha256.is_some());
        if mode == 0 {
            assert_eq!(job.segments.len(), 4);
            assert!(state.ranges.load(Ordering::SeqCst) >= 5);
        }
        task.abort();
    }
}
#[tokio::test]
async fn rejects_inconsistent_ranges() {
    for mode in [2, 3, 4] {
        let (url, _, task) = server(mode, 600_000).await;
        let dir = TempDir::new().unwrap();
        let manager = Manager::open(
            &dir.path().join("state"),
            Options {
                retries: 0,
                ..Default::default()
            },
        )
        .unwrap();
        let job = manager.add(&url, dir.path(), None).unwrap();
        manager.start(&job.id).unwrap();
        let job = wait(&manager, &job.id).await;
        assert_eq!(job.status, Status::Failed);
        assert!(!job.destination.exists());
        task.abort();
    }
}
#[tokio::test]
async fn zero_bytes_and_collision_protection() {
    let (url, _, task) = server(1, 0).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    std::fs::write(dir.path().join("file"), b"keep").unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    assert_ne!(job.destination, dir.path().join("file"));
    manager.start(&job.id).unwrap();
    assert_eq!(wait(&manager, &job.id).await.status, Status::Completed);
    assert_eq!(std::fs::read(dir.path().join("file")).unwrap(), b"keep");
    task.abort();
}
#[tokio::test]
async fn pause_resume_changed_validator_and_recovery() {
    let (url, state, task) = server(0, 2_000_000).await;
    let dir = TempDir::new().unwrap();
    let state_dir = dir.path().join("state");
    let manager = Manager::open(&state_dir, Options::default()).unwrap();
    let mut job = manager.add(&url, dir.path(), None).unwrap();
    job.bytes_per_second = 100_000;
    // Configure persistent limits through the public setter.
    manager.configure(&job.id, 0, 100_000).unwrap();
    manager.start(&job.id).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        while manager.get(&job.id).unwrap().downloaded == 0 {
            assert_ne!(manager.get(&job.id).unwrap().status, Status::Failed);
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    manager.pause(&job.id).unwrap();
    let paused = wait(&manager, &job.id).await;
    assert!(paused.downloaded > 0);
    assert_eq!(paused.status, Status::Paused);
    drop(manager);
    let manager = Manager::open(&state_dir, Options::default()).unwrap();
    assert_eq!(manager.get(&job.id).unwrap().downloaded, paused.downloaded);
    state.version.store(2, Ordering::SeqCst);
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(!failed.destination.exists());
    manager.configure(&job.id, 0, 0).unwrap();
    manager.restart(&job.id).unwrap();
    assert_eq!(wait(&manager, &job.id).await.status, Status::Completed);
    task.abort();
}
#[test]
fn metadata_and_lock_security() {
    use strata_core::{network::parse_range, sanitize_filename};
    assert_eq!(sanitize_filename("../../CON"), "_.._CON");
    assert_eq!(sanitize_filename("CON.txt"), "download_CON.txt");
    assert!(parse_range("bytes 0-10/10").is_err());
    assert!(parse_range("bytes 4294967296-4294967297/8589934592").is_ok());
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(dir.path(), Options::default()).unwrap();
    assert!(Manager::open(dir.path(), Options::default()).is_err());
    assert!(manager.add("file:///etc/passwd", dir.path(), None).is_err());
    assert!(
        manager
            .add("https://user:pass@example.org/file", dir.path(), None)
            .is_err()
    );
}

#[tokio::test]
async fn range_retries_and_rate_limits() {
    for mode in [7, 11] {
        let (url, state, task) = server(mode, 800_000).await;
        let dir = TempDir::new().unwrap();
        let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
        let job = manager.add(&url, dir.path(), None).unwrap();
        manager.start(&job.id).unwrap();
        let finished = wait(&manager, &job.id).await;
        assert_eq!(finished.status, Status::Completed, "{:?}", finished.error);
        assert_eq!(std::fs::read(finished.destination).unwrap(), *state.data);
        assert!(state.failures.load(Ordering::SeqCst) > 0);
        task.abort();
    }
}
#[tokio::test]
async fn missing_validator_uses_single_stream() {
    let (url, state, task) = server(8, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    let finished = wait(&manager, &job.id).await;
    assert_eq!(finished.status, Status::Completed, "{:?}", finished.error);
    assert_eq!(finished.segments.len(), 1);
    assert_eq!(std::fs::read(finished.destination).unwrap(), *state.data);
    task.abort();
}
async fn pause_after_progress(manager: &Manager, id: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        while manager.get(id).unwrap().downloaded == 0 {
            assert_ne!(manager.get(id).unwrap().status, Status::Failed);
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    manager.pause(id).unwrap();
    wait(manager, id).await;
}
#[tokio::test]
async fn resume_preserves_bytes_and_validates_last_modified() {
    for mode in [0, 9] {
        let (url, state, task) = server(mode, 600_000).await;
        let dir = TempDir::new().unwrap();
        let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
        let job = manager.add(&url, dir.path(), None).unwrap();
        manager.configure(&job.id, 0, 100_000).unwrap();
        manager.start(&job.id).unwrap();
        pause_after_progress(&manager, &job.id).await;
        manager.configure(&job.id, 0, 0).unwrap();
        manager.start(&job.id).unwrap();
        let finished = wait(&manager, &job.id).await;
        assert_eq!(finished.status, Status::Completed, "{:?}", finished.error);
        assert_eq!(std::fs::read(finished.destination).unwrap(), *state.data);
        task.abort();
    }
}
#[tokio::test]
async fn changed_last_modified_rejects_resume() {
    let (url, state, task) = server(9, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.configure(&job.id, 0, 100_000).unwrap();
    manager.start(&job.id).unwrap();
    pause_after_progress(&manager, &job.id).await;
    state.version.store(2, Ordering::SeqCst);
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(failed.error.unwrap().contains("version"));
    task.abort();
}
#[tokio::test]
async fn modified_partial_data_rejects_resume() {
    use std::io::{Seek, Write};
    let (url, _, task) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.configure(&job.id, 0, 100_000).unwrap();
    manager.start(&job.id).unwrap();
    pause_after_progress(&manager, &job.id).await;
    let paused = manager.get(&job.id).unwrap();
    let segment = paused.segments.iter().find(|s| s.done > 0).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join(format!(".strata-{}.part", job.id)))
        .unwrap();
    file.seek(std::io::SeekFrom::Start(segment.start)).unwrap();
    file.write_all(b"corrupt").unwrap();
    file.sync_all().unwrap();
    drop(file);
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(failed.error.unwrap().contains("checksum"));
    task.abort();
}
#[tokio::test]
async fn output_created_during_transfer_is_preserved() {
    let (url, _, task) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    std::fs::write(&job.destination, b"owned by user").unwrap();
    manager.start(&job.id).unwrap();
    assert_eq!(wait(&manager, &job.id).await.status, Status::Failed);
    assert_eq!(std::fs::read(job.destination).unwrap(), b"owned by user");
    task.abort();
}
#[tokio::test]
async fn missing_destination_reports_disk_error() {
    let (url, _, task) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out");
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, &output, None).unwrap();
    std::fs::remove_dir(output).unwrap();
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(failed.error.is_some());
    task.abort();
}
#[tokio::test]
async fn bandwidth_limit_delays_real_transfer() {
    let (url, _, task) = server(0, 64 * 1024).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.configure(&job.id, 0, 128 * 1024).unwrap();
    let started = std::time::Instant::now();
    manager.start(&job.id).unwrap();
    assert_eq!(wait(&manager, &job.id).await.status, Status::Completed);
    assert!(started.elapsed() >= std::time::Duration::from_millis(480));
    task.abort();
}
#[tokio::test]
async fn queue_priority_and_concurrency_cap() {
    let (url, _, task) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(
        &dir.path().join("state"),
        Options {
            concurrent_jobs: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let low = manager.add(&url, dir.path(), Some("low")).unwrap();
    let high = manager.add(&url, dir.path(), Some("high")).unwrap();
    manager.configure(&low.id, 0, 50_000).unwrap();
    manager.configure(&high.id, 10, 50_000).unwrap();
    manager.start(&low.id).unwrap();
    manager.start(&high.id).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while manager.get(&high.id).unwrap().status == Status::Queued {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(manager.get(&high.id).unwrap().status, Status::Downloading);
    assert_eq!(manager.get(&low.id).unwrap().status, Status::Queued);
    manager.cancel(&high.id).unwrap();
    manager.cancel(&low.id).unwrap();
    wait(&manager, &low.id).await;
    task.abort();
}
#[tokio::test]
async fn concurrent_jobs_produce_correct_files() {
    let (url, state, task) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let mut ids = vec![];
    for index in 0..5 {
        let job = manager
            .add(&url, dir.path(), Some(&format!("file-{index}")))
            .unwrap();
        manager.start(&job.id).unwrap();
        ids.push(job.id);
    }
    wait(&manager, &ids[0]).await;
    for id in ids {
        let job = manager.get(&id).unwrap();
        assert_eq!(job.status, Status::Completed, "{:?}", job.error);
        assert_eq!(std::fs::read(job.destination).unwrap(), *state.data);
    }
    task.abort();
}
#[tokio::test]
async fn redirect_limit_is_enforced() {
    let (url, state, task) = server(0, 100_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager
        .add(&url.replace("/file", "/redirect"), dir.path(), None)
        .unwrap();
    manager.start(&job.id).unwrap();
    let job = wait(&manager, &job.id).await;
    assert_eq!(job.status, Status::Completed);
    assert_eq!(std::fs::read(job.destination).unwrap(), *state.data);
    task.abort();
    let (url, _, task) = server(6, 100).await;
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    assert_eq!(wait(&manager, &job.id).await.status, Status::Failed);
    task.abort();
}
#[test]
fn corrupted_database_snapshot_is_rejected() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(dir.path(), Options::default()).unwrap();
    manager
        .add("https://example.org/file", dir.path(), None)
        .unwrap();
    drop(manager);
    let connection = rusqlite::Connection::open(dir.path().join("state.sqlite")).unwrap();
    connection
        .execute("UPDATE jobs SET digest='wrong'", [])
        .unwrap();
    drop(connection);
    assert!(Manager::open(dir.path(), Options::default()).is_err());
}

#[test]
fn newer_database_schema_is_not_downgraded() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(dir.path(), Options::default()).unwrap();
    drop(manager);
    let connection = rusqlite::Connection::open(dir.path().join("state.sqlite")).unwrap();
    connection.execute_batch("PRAGMA user_version=99;").unwrap();
    drop(connection);
    assert!(Manager::open(dir.path(), Options::default()).is_err());
    let connection = rusqlite::Connection::open(dir.path().join("state.sqlite")).unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 99);
}
#[test]
fn preview_schema_migrates_without_losing_jobs() {
    use sha2::{Digest, Sha256};
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(dir.path(), Options::default()).unwrap();
    let job = manager
        .add("https://example.org/report.pdf", dir.path(), None)
        .unwrap();
    drop(manager);
    let connection = rusqlite::Connection::open(dir.path().join("state.sqlite")).unwrap();
    let mut legacy = serde_json::to_value(&job).unwrap();
    legacy.as_object_mut().unwrap().remove("category");
    legacy.as_object_mut().unwrap().remove("scheduled_at");
    let json = serde_json::to_string(&legacy).unwrap();
    let digest = format!("{:x}", Sha256::digest(json.as_bytes()));
    connection
        .execute(
            "UPDATE jobs SET json=?1,digest=?2 WHERE id=?3",
            rusqlite::params![json, digest, job.id],
        )
        .unwrap();
    connection
        .execute_batch("DROP TABLE settings; PRAGMA user_version=1;")
        .unwrap();
    drop(connection);
    let manager = Manager::open(dir.path(), Options::default()).unwrap();
    let recovered = manager.get(&job.id).unwrap();
    assert_eq!(recovered.url, job.url);
    assert_eq!(recovered.destination, job.destination);
    assert_eq!(recovered.category, "Documents");
    assert_eq!(recovered.status, Status::Paused);
    drop(manager);
    let connection = rusqlite::Connection::open(dir.path().join("state.sqlite")).unwrap();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 2);
}
#[tokio::test]
async fn interrupted_publication_recovers_from_same_file_links() {
    use sha2::{Digest, Sha256};
    let (url, _, task) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let state_path = dir.path().join("state");
    let manager = Manager::open(&state_path, Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    let mut complete = wait(&manager, &job.id).await;
    drop(manager);
    let stage = dir.path().join(format!(".strata-{}.part", job.id));
    std::fs::hard_link(&complete.destination, &stage).unwrap();
    complete.status = Status::Downloading;
    complete.sha256 = None;
    complete.completed = None;
    let json = serde_json::to_string(&complete).unwrap();
    let digest = format!("{:x}", Sha256::digest(json.as_bytes()));
    let connection = rusqlite::Connection::open(state_path.join("state.sqlite")).unwrap();
    connection
        .execute(
            "UPDATE jobs SET json=?1,digest=?2 WHERE id=?3",
            rusqlite::params![json, digest, job.id],
        )
        .unwrap();
    drop(connection);
    let manager = Manager::open(&state_path, Options::default()).unwrap();
    let recovered = manager.get(&job.id).unwrap();
    assert_eq!(recovered.status, Status::Completed);
    assert!(recovered.sha256.is_some());
    assert!(!stage.exists());
    task.abort();
}

#[tokio::test]
async fn completed_checksum_detects_file_changes() {
    let (url, _, task) = server(0, 100_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    let complete = wait(&manager, &job.id).await;
    manager.verify(&job.id, None).unwrap();
    assert!(manager.verify(&job.id, Some(&"0".repeat(64))).is_err());
    std::fs::write(complete.destination, b"changed after completion").unwrap();
    assert!(manager.verify(&job.id, None).is_err());
    task.abort();
}

#[tokio::test]
async fn extreme_untrusted_range_total_fails_without_panicking() {
    let (url, _, task) = server(13, 100).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(!failed.destination.exists());
    assert!(!manager.is_running());
    task.abort();
}

#[tokio::test]
async fn missing_and_truncated_checkpoints_never_publish_and_restart_repairs() {
    for missing in [true, false] {
        let (url, data, server) = server(0, 600_000).await;
        let dir = TempDir::new().unwrap();
        let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
        let job = manager.add(&url, dir.path(), None).unwrap();
        manager.configure(&job.id, 0, 100_000).unwrap();
        manager.start(&job.id).unwrap();
        pause_after_progress(&manager, &job.id).await;
        let stage = dir.path().join(format!(".strata-{}.part", job.id));
        if missing {
            std::fs::remove_file(&stage).unwrap();
        } else {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&stage)
                .unwrap()
                .set_len(1)
                .unwrap();
        }
        manager.start(&job.id).unwrap();
        let failed = wait(&manager, &job.id).await;
        assert_eq!(failed.status, Status::Failed, "{:?}", failed.error);
        assert!(!job.destination.exists());
        manager.configure(&job.id, 0, 0).unwrap();
        manager.restart(&job.id).unwrap();
        let completed = wait(&manager, &job.id).await;
        assert_eq!(completed.status, Status::Completed, "{:?}", completed.error);
        assert_eq!(std::fs::read(&completed.destination).unwrap(), *data.data);
        manager.verify(&job.id, None).unwrap();
        server.abort();
    }
}

#[cfg(windows)]
#[tokio::test]
async fn windows_readonly_staging_fails_without_modifying_existing_bytes() {
    let (url, _, server) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    let stage = dir.path().join(format!(".strata-{}.part", job.id));
    std::fs::write(&stage, b"protected").unwrap();
    let mut permissions = std::fs::metadata(&stage).unwrap().permissions();
    let original_permissions = permissions.clone();
    permissions.set_readonly(true);
    std::fs::set_permissions(&stage, permissions).unwrap();
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(!job.destination.exists());
    assert_eq!(std::fs::read(&stage).unwrap(), b"protected");
    std::fs::set_permissions(&stage, original_permissions).unwrap();
    server.abort();
}

#[tokio::test]
async fn scheduled_queue_stops_immediately_and_dispatcher_can_restart() {
    let (url, data, server) = server(0, 1024).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager
        .schedule(&job.id, "fixture", Some(u64::MAX))
        .unwrap();
    manager.start(&job.id).unwrap();
    manager.pause(&job.id).unwrap();
    manager.schedule(&job.id, "fixture", None).unwrap();
    manager.start(&job.id).unwrap();
    let completed = wait(&manager, &job.id).await;
    assert_eq!(completed.status, Status::Completed, "{:?}", completed.error);
    assert_eq!(std::fs::read(completed.destination).unwrap(), *data.data);
    server.abort();
}

#[tokio::test]
async fn damage_during_active_transfer_is_not_blessed_by_final_checksum() {
    use std::io::{Seek, Write};
    let (url, _, server) = server(0, 600_000).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.configure(&job.id, 0, 200_000).unwrap();
    manager.start(&job.id).unwrap();
    let progress = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let current = manager.get(&job.id).unwrap();
            if current.downloaded > 0 {
                break current;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let prefix = progress.segments.iter().find(|s| s.done > 0).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join(format!(".strata-{}.part", job.id)))
        .unwrap();
    file.seek(std::io::SeekFrom::Start(prefix.start)).unwrap();
    file.write_all(b"damaged").unwrap();
    file.sync_all().unwrap();
    drop(file);
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed, "{:?}", failed.error);
    assert!(failed.error.unwrap().contains("checksum"));
    assert!(!job.destination.exists());
    server.abort();
}

#[cfg(windows)]
#[tokio::test]
async fn replaced_destination_with_ntfs_junction_is_rejected() {
    let (url, _, server) = server(0, 1024).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let output = dir.path().join("output");
    let redirected = dir.path().join("redirected");
    let job = manager.add(&url, &output, None).unwrap();
    std::fs::remove_dir(&output).unwrap();
    std::fs::create_dir(&redirected).unwrap();
    let quote = |p: &std::path::Path| format!("'{}'", p.to_string_lossy().replace('\'', "''"));
    let result = std::process::Command::new("powershell.exe").args([
        "-NoProfile", "-NonInteractive", "-Command",
        &format!("$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path {} -Target {} | Out-Null", quote(&output), quote(&redirected)),
    ]).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    manager.start(&job.id).unwrap();
    let failed = wait(&manager, &job.id).await;
    assert_eq!(failed.status, Status::Failed);
    assert!(failed.error.unwrap().contains("redirected"));
    assert_eq!(std::fs::read_dir(&redirected).unwrap().count(), 0);
    std::fs::remove_dir(&output).unwrap(); // Remove only the junction, not its target.
    server.abort();
}
