//! Repeatable local-fixture completion timings. No universal speed claim.
use strata_core::{Manager, Options, Status};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:18765/fixture.bin".into());
    println!("connections,trial,bytes,milliseconds,sha256");
    for connections in [1, 4] {
        for trial in 1..=5 {
            let dir = tempfile::tempdir()?;
            let manager = Manager::open(
                &dir.path().join("state"),
                Options {
                    connections,
                    ..Default::default()
                },
            )?;
            let job = manager.add(&url, &dir.path().join("out"), None)?;
            let start = std::time::Instant::now();
            manager.start(&job.id)?;
            tokio::time::timeout(std::time::Duration::from_secs(60), async {
                while manager.is_running() {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
            .await?;
            let job = manager.get(&job.id)?;
            anyhow::ensure!(
                job.status == Status::Completed,
                "Download failed: {:?}",
                job.error
            );
            anyhow::ensure!(
                job.sha256.as_deref()
                    == Some("bdf23837181f5808331800c1ae2b4f7d7a839536b10d58491471c50dde23833a"),
                "Fixture bytes differ"
            );
            println!(
                "{connections},{trial},{},{:.3},{}",
                job.downloaded,
                start.elapsed().as_secs_f64() * 1000.0,
                job.sha256.unwrap()
            );
        }
    }
    Ok(())
}
