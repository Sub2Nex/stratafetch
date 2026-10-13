//! Local persisted-history/queue/IPC benchmark. No remote network or CI.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use strata_app::{Command, ipc};
use strata_core::{Manager, Options};

fn percentiles(mut samples: Vec<f64>) -> serde_json::Value {
    samples.sort_by(f64::total_cmp);
    serde_json::json!({"p50_ms": samples[samples.len()/2], "p95_ms": samples[(samples.len()*95/100).min(samples.len()-1)]})
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(std::env::args().nth(1).expect("isolated output directory"));
    std::fs::create_dir_all(&root)?;
    let mut reports = Vec::new();
    for count in [100u64, 1000, 10000] {
        let directory = root.join(format!("records-{count}"));
        let seed = Instant::now();
        let template = {
            let manager = Manager::open(&directory, Options::default())?;
            anyhow::ensure!(manager.list().is_empty(), "Benchmark requires fresh state");
            manager.add(
                "https://example.invalid/fixture.bin",
                &directory.join("downloads"),
                None,
            )?
        };
        {
            // Real SQLite records, seeded without quadratic Manager.add setup cost.
            let store = strata_core::storage::Store::open(&directory)?;
            for i in 1..count {
                let mut job = template.clone();
                job.id = format!("benchmark-{i}");
                job.sequence = i + 1;
                job.destination = template
                    .destination
                    .parent()
                    .unwrap()
                    .join(format!("file-{i}.bin"));
                store.save(&job)?;
            }
        }
        let seed_ms = seed.elapsed().as_secs_f64() * 1000.0;
        let started = Instant::now();
        let manager = Manager::open(&directory, Options::default())?;
        let reopen_ms = started.elapsed().as_secs_f64() * 1000.0;
        let jobs = manager.list();
        anyhow::ensure!(
            jobs.len() == count as usize,
            "Persisted record count mismatch"
        );
        let snapshot_bytes = serde_json::to_vec(&jobs)?.len();
        let service = tokio::spawn({
            let manager = manager.clone();
            let directory = directory.clone();
            async move { ipc::serve(&directory, manager).await }
        });
        for _ in 0..100 {
            if ipc::call(&directory, Command::List).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut lists = Vec::new();
        let mut ipc_samples = Vec::new();
        for _ in 0..20 {
            let started = Instant::now();
            let list = manager.list();
            lists.push(started.elapsed().as_secs_f64() * 1000.0);
            anyhow::ensure!(list.len() == jobs.len(), "Snapshot count mismatch");
            let started = Instant::now();
            let reply = ipc::call(&directory, Command::List).await?;
            ipc_samples.push(started.elapsed().as_secs_f64() * 1000.0);
            anyhow::ensure!(
                reply.error.is_none()
                    && reply
                        .jobs
                        .iter()
                        .map(|j| &j.id)
                        .eq(jobs.iter().map(|j| &j.id)),
                "IPC snapshot mismatch"
            );
        }
        let mut queue = Vec::new();
        for job in &jobs {
            manager.schedule(&job.id, "fixture", Some(u64::MAX))?;
            let started = Instant::now();
            manager.start(&job.id)?;
            queue.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        std::fs::write(
            root.join("stage.json"),
            serde_json::to_vec(&serde_json::json!({"count":count,"stage":"queued_idle"}))?,
        )?;
        tokio::time::sleep(Duration::from_secs(6)).await;
        // End the idle sample before durable pause writes start.
        std::fs::write(root.join("stage.json"), b"{}")?;
        let mut pause = Vec::new();
        for job in &jobs {
            let started = Instant::now();
            manager.pause(&job.id)?;
            pause.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        strata_app::shutdown(&manager).await;
        reports.push(serde_json::json!({"records":count,"seed_ms":seed_ms,"reopen_ms":reopen_ms,"snapshot_json_bytes":snapshot_bytes,"list":percentiles(lists),"ipc_list":percentiles(ipc_samples),"queue_start":percentiles(queue),"queue_pause":percentiles(pause)}));
        std::fs::write(root.join("stage.json"), b"{}")?;
        service.abort();
        let _ = service.await;
        drop(manager);
    }
    std::fs::write(
        root.join("latency.json"),
        serde_json::to_vec_pretty(&reports)?,
    )?;
    Ok(())
}
