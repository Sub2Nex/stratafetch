use strata_app::{Command, ipc};
use strata_core::{Manager, Options};
#[tokio::test]
async fn os_local_roundtrip_and_exclusive_state() {
    let directory = tempfile::tempdir().unwrap();
    let manager = Manager::open(directory.path(), Options::default()).unwrap();
    let job = manager
        .add("https://example.org/file", directory.path(), None)
        .unwrap();
    let service = tokio::spawn({
        let directory = directory.path().to_path_buf();
        let manager = manager.clone();
        async move { ipc::serve(&directory, manager).await }
    });
    let reply = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match ipc::call(directory.path(), Command::List).await {
                Ok(reply) => break reply,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(reply.version, 1);
    assert_eq!(reply.jobs.len(), 1);
    assert_eq!(reply.jobs[0].id, job.id);
    let reply = ipc::call(
        directory.path(),
        Command::Configure {
            id: job.id.clone(),
            priority: 5,
            bytes_per_second: 12345,
        },
    )
    .await
    .unwrap();
    assert!(reply.error.is_none());
    assert_eq!(reply.jobs[0].bytes_per_second, 12345);
    let reply = ipc::call(directory.path(), Command::Remove { id: job.id })
        .await
        .unwrap();
    assert!(reply.error.is_none());
    assert!(reply.jobs.is_empty());
    service.abort();
}

#[tokio::test]
async fn large_history_and_mutation_replies_cross_multiple_frames() {
    let directory = tempfile::tempdir().unwrap();
    let template = {
        let manager = Manager::open(directory.path(), Options::default()).unwrap();
        manager
            .add("https://example.org/file", directory.path(), None)
            .unwrap()
    };
    {
        let store = strata_core::storage::Store::open(directory.path()).unwrap();
        for i in 0..4000 {
            let mut job = template.clone();
            job.id = format!("fixture-{i}");
            job.sequence = i + 2;
            job.destination = directory.path().join(format!("file-{i}"));
            store.save(&job).unwrap();
        }
    }
    let manager = Manager::open(directory.path(), Options::default()).unwrap();
    let expected = manager.list();
    assert!(serde_json::to_vec(&expected).unwrap().len() > 1024 * 1024);
    let service = tokio::spawn({
        let directory = directory.path().to_path_buf();
        let manager = manager.clone();
        async move { ipc::serve(&directory, manager).await }
    });
    let reply = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            match ipc::call(directory.path(), Command::List).await {
                Ok(reply) => break reply,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(
        reply.jobs.iter().map(|j| &j.id).collect::<Vec<_>>(),
        expected.iter().map(|j| &j.id).collect::<Vec<_>>()
    );
    let reply = ipc::call(
        directory.path(),
        Command::Configure {
            id: template.id.clone(),
            priority: 7,
            bytes_per_second: 123,
        },
    )
    .await
    .unwrap();
    assert!(reply.error.is_none());
    assert_eq!(reply.jobs.len(), 4001);
    assert_eq!(reply.jobs[0].id, template.id);
    assert_eq!(reply.jobs[0].priority, 7);
    service.abort();
}
