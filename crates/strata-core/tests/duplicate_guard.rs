use strata_core::{Manager, Options, Status};

#[test]
fn duplicate_manual_additions_share_one_reserved_job_even_when_added_concurrently() {
    let temp = tempfile::tempdir().unwrap();
    let manager = Manager::open(&temp.path().join("state"), Options::default()).unwrap();
    let output = temp.path().join("downloads");
    let ids = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let manager = manager.clone();
                let output = output.clone();
                scope.spawn(move || {
                    manager
                        .add_unique("https://example.org/file.zip", &output, None)
                        .unwrap()
                        .0
                        .id
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<std::collections::HashSet<_>>()
    });
    assert_eq!(ids.len(), 1);
    assert_eq!(manager.list().len(), 1);
    assert_eq!(manager.list()[0].status, Status::Paused);
    let renamed = manager
        .add_unique("https://example.org/file.zip", &output, Some("other.zip"))
        .unwrap()
        .0;
    assert!(
        !ids.contains(&renamed.id),
        "an explicit different filename remains a separate download"
    );
}

#[test]
fn completed_files_are_preserved_and_numbered_retries_are_deduplicated() {
    let temp = tempfile::tempdir().unwrap();
    let manager = Manager::open(&temp.path().join("state"), Options::default()).unwrap();
    let output = temp.path().join("downloads");
    std::fs::create_dir(&output).unwrap();
    let file = output.join("file.zip");
    std::fs::write(&file, b"keep me").unwrap();
    let completed = manager
        .adopt_completed("https://example.org/file.zip", &file, 7)
        .unwrap();
    let next = manager
        .add_unique("https://example.org/file.zip", &output, None)
        .unwrap()
        .0;
    let repeated = manager
        .add_unique("https://example.org/file.zip", &output, None)
        .unwrap()
        .0;
    assert_ne!(next.id, completed.id);
    assert_eq!(next.id, repeated.id);
    assert_eq!(std::fs::read(file).unwrap(), b"keep me");
    assert_eq!(manager.list().len(), 2);
}
