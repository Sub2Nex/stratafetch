use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Response},
    routing::get,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use strata_core::{Manager, Options, Settings, Status};
use tempfile::TempDir;

async fn fixture(transient_probe: bool) -> (String, Vec<u8>, tokio::task::JoinHandle<()>) {
    let data: Vec<u8> = (0..512 * 1024).map(|i| (i % 251) as u8).collect();
    let payload = Arc::new(data.clone());
    let requests = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route(
        "/file",
        get(move |headers: HeaderMap| {
            let payload = payload.clone();
            let requests = requests.clone();
            async move {
                if transient_probe && requests.fetch_add(1, Ordering::SeqCst) == 0 {
                    return Response::builder()
                        .status(503)
                        .header("Retry-After", "0")
                        .body(Body::empty())
                        .unwrap();
                }
                if let Some(range) = headers.get("range") {
                    let (first, last) = range
                        .to_str()
                        .unwrap()
                        .strip_prefix("bytes=")
                        .unwrap()
                        .split_once('-')
                        .unwrap();
                    let first: usize = first.parse().unwrap();
                    let last: usize = last.parse::<usize>().unwrap().min(payload.len() - 1);
                    return Response::builder()
                        .status(206)
                        .header("ETag", "\"beta-fixture\"")
                        .header(
                            "Content-Range",
                            format!("bytes {first}-{last}/{}", payload.len()),
                        )
                        .body(Body::from(payload[first..=last].to_vec()))
                        .unwrap();
                }
                Response::builder()
                    .header("ETag", "\"beta-fixture\"")
                    .body(Body::from(payload.as_ref().clone()))
                    .unwrap()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}/file"), data, server)
}
async fn idle(manager: &Manager) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while manager.is_running() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[test]
fn url_filename_is_decoded_for_cli_and_browser_adds() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager
        .add(
            "https://example.org/r%C3%A4ksm%C3%B6rg%C3%A5s%20final.zip",
            dir.path(),
            None,
        )
        .unwrap();
    assert_eq!(job.destination.file_name().unwrap(), "räksmörgås final.zip");
}
#[test]
fn unicode_filenames_fit_the_real_filesystem() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    for name in [
        format!("{}.zip", "🦀".repeat(170)),
        format!("{}.pdf", "ä".repeat(170)),
    ] {
        let job = manager
            .add("https://example.org/file", dir.path(), Some(&name))
            .unwrap();
        assert!(job.destination.file_name().unwrap().to_string_lossy().len() <= 180);
        std::fs::File::create(job.destination).unwrap();
    }
}
#[test]
fn windows_device_aliases_are_not_download_names() {
    for name in [
        "COM¹.txt",
        "COM²",
        "COM³.bin",
        "LPT¹",
        "LPT².txt",
        "LPT³",
        "CONIN$",
        "CONOUT$",
        "NUL .txt",
    ] {
        assert_ne!(
            strata_core::sanitize_filename(name),
            name,
            "{name} is a Windows device alias"
        );
    }
}
#[test]
fn histories_over_one_thousand_entries_roundtrip() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    for index in 0..1001 {
        manager
            .add(
                &format!("https://example.org/file-{index}"),
                &dir.path().join("out"),
                None,
            )
            .unwrap();
    }
    let export = dir.path().join("history.json");
    manager.export_history(&export).unwrap();
    let imported = Manager::open(&dir.path().join("import-state"), Options::default()).unwrap();
    imported
        .import_history(&export, &dir.path().join("import-out"))
        .unwrap();
    assert_eq!(imported.list().len(), 1001);
    assert!(
        imported
            .list()
            .iter()
            .all(|job| job.status == Status::Paused)
    );
}
#[test]
fn unicode_history_categories_roundtrip() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager
        .add("https://example.org/file", dir.path(), None)
        .unwrap();
    manager.schedule(&job.id, &"ä".repeat(120), None).unwrap();
    let category = manager.get(&job.id).unwrap().category;
    let export = dir.path().join("history.json");
    manager.export_history(&export).unwrap();
    let imported = Manager::open(&dir.path().join("import-state"), Options::default()).unwrap();
    imported
        .import_history(&export, &dir.path().join("import-out"))
        .unwrap();
    assert_eq!(imported.list()[0].category, category);
}
#[tokio::test]
async fn transient_probe_failures_honor_the_retry_budget() {
    let (url, bytes, server) = fixture(true).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    idle(&manager).await;
    let job = manager.get(&job.id).unwrap();
    assert_eq!(job.status, Status::Completed, "{:?}", job.error);
    assert_eq!(std::fs::read(job.destination).unwrap(), bytes);
    server.abort();
}
#[tokio::test]
async fn immediate_resume_after_pause_is_not_lost() {
    let (url, bytes, server) = fixture(false).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    manager
        .set_settings(Settings {
            global_bytes_per_second: 128 * 1024,
            ..Default::default()
        })
        .unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while manager.get(&job.id).unwrap().downloaded == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    manager.pause(&job.id).unwrap();
    manager.start(&job.id).unwrap();
    manager.set_settings(Settings::default()).unwrap();
    idle(&manager).await;
    let job = manager.get(&job.id).unwrap();
    assert_eq!(
        job.status,
        Status::Completed,
        "immediate Resume was silently ignored"
    );
    assert_eq!(std::fs::read(job.destination).unwrap(), bytes);
    server.abort();
}
#[test]
fn oversized_urls_do_not_create_unreadable_ipc_records() {
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    let url = format!(
        "https://example.org/file?signature={}",
        "a".repeat(600 * 1024)
    );
    assert!(manager.add(&url, &dir.path().join("out"), None).is_err());
    assert!(strata_core::metadata::identity(&url).is_err());
    assert!(manager.list().is_empty());
    assert!(!dir.path().join("out").exists());
}
#[cfg(unix)]
#[test]
fn private_history_exports_protect_signed_url_parameters() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    manager
        .add(
            "https://example.org/file?signature=private",
            dir.path(),
            None,
        )
        .unwrap();
    let export = dir.path().join("history.json");
    manager.export_history(&export).unwrap();
    assert_eq!(
        std::fs::metadata(export).unwrap().permissions().mode() & 0o077,
        0
    );
}
#[cfg(unix)]
#[test]
fn state_database_is_private_even_with_a_permissive_umask() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let state = dir.path().join("state");
    let manager = Manager::open(&state, Options::default()).unwrap();
    manager
        .add(
            "https://example.org/file?signature=private",
            dir.path(),
            None,
        )
        .unwrap();
    for name in ["state.sqlite", "state.sqlite-wal", "state.sqlite-shm"] {
        let file = state.join(name);
        if file.exists() {
            assert_eq!(
                std::fs::metadata(file).unwrap().permissions().mode() & 0o077,
                0,
                "{name} exposes private state"
            );
        }
    }
}
#[cfg(unix)]
#[test]
fn state_database_symlinks_cannot_alias_another_profile() {
    let dir = TempDir::new().unwrap();
    let source = dir.path().join("source");
    let manager = Manager::open(&source, Options::default()).unwrap();
    manager
        .add("https://example.org/private", dir.path(), None)
        .unwrap();
    drop(manager);
    let alias = dir.path().join("alias");
    std::fs::create_dir(&alias).unwrap();
    std::os::unix::fs::symlink(source.join("state.sqlite"), alias.join("state.sqlite")).unwrap();
    assert!(
        Manager::open(&alias, Options::default()).is_err(),
        "a separate lockfile must not bypass the original profile lock through a database symlink"
    );
    assert_eq!(
        Manager::open(&source, Options::default())
            .unwrap()
            .list()
            .len(),
        1
    );
}
#[tokio::test]
async fn pausing_again_cancels_a_pending_immediate_resume() {
    let (url, _, server) = fixture(false).await;
    let dir = TempDir::new().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Options::default()).unwrap();
    manager
        .set_settings(Settings {
            global_bytes_per_second: 1,
            ..Default::default()
        })
        .unwrap();
    let job = manager.add(&url, dir.path(), None).unwrap();
    manager.start(&job.id).unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while manager.get(&job.id).unwrap().status != Status::Downloading {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    manager.pause(&job.id).unwrap();
    manager.start(&job.id).unwrap();
    manager.pause(&job.id).unwrap();
    idle(&manager).await;
    assert_eq!(manager.get(&job.id).unwrap().status, Status::Paused);
    assert!(!job.destination.exists());
    server.abort();
}
