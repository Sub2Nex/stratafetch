use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Response},
    routing::get,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use strata_core::{Manager, Options, Status};

async fn fixture(broken_probes: usize) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let probes = Arc::new(AtomicUsize::new(0));
    let observed = probes.clone();
    let app = Router::new().route(
        "/file",
        get(move |headers: HeaderMap| {
            let probes = observed.clone();
            async move {
                let raw = headers.get("range").unwrap().to_str().unwrap();
                let (start, end) = raw.strip_prefix("bytes=").unwrap().split_once('-').unwrap();
                let start: usize = start.parse().unwrap();
                let end: usize = end.parse().unwrap();
                let mut response = Response::builder()
                    .status(206)
                    .header("ETag", "\"stability\"")
                    .header("Content-Range", format!("bytes {start}-{end}/65536"));
                if end == 0 && probes.fetch_add(1, Ordering::SeqCst) < broken_probes {
                    // Valid headers followed by a dropped/truncated probe body.
                    response = response.header("Content-Length", "1");
                    return response
                        .body(Body::from_stream(futures_util::stream::once(async {
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            Err::<Vec<u8>, _>(std::io::Error::new(
                                std::io::ErrorKind::ConnectionReset,
                                "probe disconnected",
                            ))
                        })))
                        .unwrap();
                }
                response
                    .body(Body::from(vec![42u8; end - start + 1]))
                    .unwrap()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (url, probes, server)
}

async fn idle(manager: &Manager) {
    tokio::time::timeout(Duration::from_secs(8), async {
        while manager.is_running() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn truncated_probe_body_is_retried_within_budget() {
    for (broken, retries, expected) in [(1, 2, Status::Completed), (4, 1, Status::Failed)] {
        let (url, probes, server) = fixture(broken).await;
        let temp = tempfile::tempdir().unwrap();
        let manager = Manager::open(
            &temp.path().join("state"),
            Options {
                retries,
                ..Default::default()
            },
        )
        .unwrap();
        let job = manager.add(&url, temp.path(), None).unwrap();
        manager.start(&job.id).unwrap();
        idle(&manager).await;
        let job = manager.get(&job.id).unwrap();
        assert_eq!(job.status, expected, "{:?}", job.error);
        assert_eq!(probes.load(Ordering::SeqCst), 2);
        if expected == Status::Completed {
            assert_eq!(std::fs::read(job.destination).unwrap(), vec![42; 65536]);
        } else {
            assert!(!job.destination.exists());
        }
        server.abort();
    }
}

#[tokio::test]
async fn slow_limits_report_progress_and_pause_without_a_large_burst() {
    for global in [false, true] {
        let (url, _, server) = fixture(0).await;
        let temp = tempfile::tempdir().unwrap();
        let manager = Manager::open(&temp.path().join("state"), Options::default()).unwrap();
        let job = manager.add(&url, temp.path(), None).unwrap();
        if global {
            manager
                .set_settings(strata_core::Settings {
                    global_bytes_per_second: 1024,
                    ..Default::default()
                })
                .unwrap();
        } else {
            manager.configure(&job.id, 0, 1024).unwrap();
        }
        manager.start(&job.id).unwrap();
        let progress = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let job = manager.get(&job.id).unwrap();
                assert_ne!(job.status, Status::Failed, "{:?}", job.error);
                if job.downloaded > 0 {
                    break job.downloaded;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
        manager.pause(&job.id).unwrap();
        idle(&manager).await;
        assert!(
            progress.is_ok(),
            "limiter hid all progress for over 3 seconds (global={global})"
        );
        let paused = manager.get(&job.id).unwrap();
        assert!(
            paused.downloaded <= 4096,
            "limiter wrote a large unpaced burst: {}",
            paused.downloaded
        );
        manager.configure(&job.id, 0, 0).unwrap();
        manager.set_settings(Default::default()).unwrap();
        manager.start(&job.id).unwrap();
        idle(&manager).await;
        let finished = manager.get(&job.id).unwrap();
        assert_eq!(finished.status, Status::Completed, "{:?}", finished.error);
        assert_eq!(
            std::fs::read(finished.destination).unwrap(),
            vec![42; 65536]
        );
        server.abort();
    }
}
