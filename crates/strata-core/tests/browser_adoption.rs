use strata_core::{Manager, Options, Status};

#[test]
fn adoption_keeps_one_existing_file_and_persists_without_a_transfer() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("installer.exe");
    std::fs::write(&path, b"controlled test bytes").unwrap();
    let state = root.path().join("state");
    let manager = Manager::open(&state, Options::default()).unwrap();
    assert!(
        manager
            .adopt_completed("https://example.org/installer.exe", &path, 1)
            .is_err()
    );
    let job = manager
        .adopt_completed("https://example.org/installer.exe", &path, 21)
        .unwrap();
    assert_eq!(job.status, Status::Completed);
    assert_eq!(job.downloaded, 21);
    assert_eq!(job.destination, path.canonicalize().unwrap());
    assert_eq!(
        manager.adopt_completed(&job.url, &path, 21).unwrap().id,
        job.id
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"controlled test bytes");
    assert!(
        manager
            .adopt_completed("file:///private", &path, 21)
            .is_err()
    );
    drop(manager);
    let manager = Manager::open(&state, Options::default()).unwrap();
    assert_eq!(manager.get(&job.id).unwrap().status, Status::Completed);
    assert_eq!(std::fs::read(&path).unwrap(), b"controlled test bytes");
}
