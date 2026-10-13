//! Authentic UI fixtures: all records come from the public Manager API.
//! Run with isolated STRATAFETCH_STATE_DIR and DOWNLOAD_DIR and fixture_server.
use strata_core::{Manager, Options, Status};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let count: usize = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "24".into())
        .parse()?;
    let manager = Manager::open(
        &strata_app::state_dir(),
        Options {
            retries: 0,
            ..Default::default()
        },
    )?;
    let folder = strata_app::download_dir();
    if std::env::args().nth(2).as_deref() == Some("one") {
        manager.add(
            "http://127.0.0.1:18765/fixture.bin",
            &folder,
            Some("One download.zip"),
        )?;
        println!("Ready: one actual paused record");
        strata_app::ipc::serve(&strata_app::state_dir(), manager).await?;
        return Ok(());
    }
    for (path, name) in [
        ("fixture.bin?bytes=65536", "Release notes.pdf"),
        ("fixture.bin?bytes=131072", "Photographs.zip"),
        ("missing", "Unavailable installer.exe"),
    ] {
        let job = manager.add(
            &format!("http://127.0.0.1:18765/{path}"),
            &folder,
            Some(name),
        )?;
        manager.start(&job.id)?;
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while manager.is_running() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await?;
        let finished = manager.get(&job.id)?;
        anyhow::ensure!(
            matches!(finished.status, Status::Completed | Status::Failed),
            "fixture did not finish"
        );
    }
    for index in 0..count {
        let extension = ["zip", "pdf", "mp3", "exe", "mp4", "png"][index % 6];
        let name = if index == 0 {
            format!(
                "A very long filename for checking truncation and hover details in the compact download table.{extension}"
            )
        } else {
            format!("Download {:04}.{extension}", index + 1)
        };
        let job = manager.add(&format!("http://127.0.0.1:18765/fixture.bin?bytes=1048576&reference=an-authentic-long-source-URL-for-layout-check-{index}"), &folder, Some(&name))?;
        if index == 1 {
            manager.schedule(
                &job.id,
                "Documents",
                Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs()
                        + 3600,
                ),
            )?;
        }
    }
    manager.set_settings(strata_core::Settings {
        global_bytes_per_second: 262144,
        ..Default::default()
    })?;
    let active = manager.add(
        "http://127.0.0.1:18765/fixture.bin?bytes=67108864&slow=1",
        &folder,
        Some("Active transfer.zip"),
    )?;
    manager.start(&active.id)?;
    let queued = manager.add(
        "http://127.0.0.1:18765/fixture.bin?bytes=65536",
        &folder,
        Some("Scheduled download.zip"),
    )?;
    manager.schedule(
        &queued.id,
        "Archives",
        Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                + 3600,
        ),
    )?;
    manager.start(&queued.id)?;
    println!("Ready: {} actual download records", manager.list().len());
    strata_app::ipc::serve(&strata_app::state_dir(), manager).await?;
    Ok(())
}
