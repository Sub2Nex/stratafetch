use strata_app::{BatchAction, Command, execute};
use strata_core::{Manager, Status};
#[test]
fn later_add_is_paused_and_batch_removes_inactive_history() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Manager::open(&dir.path().join("state"), Default::default()).unwrap();
    let reply = execute(
        &manager,
        Command::AddPaused {
            url: "https://example.com/release.zip".into(),
            directory: dir.path().join("out"),
            name: Some("release.zip".into()),
            category: "Documents".into(),
        },
    );
    assert!(reply.error.is_none());
    let id = reply.accepted_id.unwrap();
    let job = manager.get(&id).unwrap();
    assert_eq!(job.status, Status::Paused);
    assert_eq!(job.category, "Documents");
    assert!(!manager.is_running());
    assert!(!job.destination.exists());
    let reply = execute(
        &manager,
        Command::Batch {
            ids: vec![id],
            action: BatchAction::Remove,
        },
    );
    assert!(reply.error.is_none());
    assert!(manager.list().is_empty());
}
