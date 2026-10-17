//! Official browser native-messaging framing. This bridge requires a running manager.
use anyhow::{Context, Result, ensure};
use strata_app::{Command, download_dir, ipc, native, state_dir};
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--version") {
        println!("stratafetch-browser-host {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "--unregister-local") {
        strata_app::browser_integration::unregister_local()?;
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|arg| matches!(arg.as_str(), "--register-local" | "--setup"))
    {
        strata_app::browser_integration::register_local()?;
        if args[0] == "--setup" {
            strata_app::browser_integration::launch_desktop(true)?;
        }
        return Ok(());
    }
    if args
        .first()
        .is_some_and(|arg| matches!(arg.as_str(), "--register" | "--unregister"))
    {
        return registration(args);
    }
    let allowlist: Vec<String> =
        serde_json::from_slice(&std::fs::read(state_dir().join("browser-origins.json"))?)?;
    ensure!(
        args.iter().any(|arg| allowlist.contains(arg)),
        "Unregistered browser extension origin"
    );
    let runtime = tokio::runtime::Runtime::new()?;
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    while let Some(mut message) = native::read_message(&mut input)? {
        let result: Result<_> = (|| {
            if message.action == "probeReplay" {
                return Ok(
                    serde_json::json!({"ok":true,"replayable":strata_app::browser_capture::replayable(&runtime, &message)}),
                );
            }
            if message.action == "capture" {
                if !strata_app::browser_capture::confirm(&runtime, &message)? {
                    return Ok(serde_json::json!({"ok":false,"declined":true}));
                }
                // Consent does not start any network activity. Give the
                // extension one final safety/pause check after the dialog,
                // and require its commit on this same authenticated port.
                native::write_message(
                    &mut output,
                    &serde_json::json!({"ok":true,"confirmation":"accepted"}),
                )?;
                let commit = native::read_message(&mut input)?
                    .context("Browser handoff ended before commit")?;
                ensure!(
                    matches!(commit.action.as_str(), "commitCapture" | "adoptCapture"),
                    "Expected browser handoff commit"
                );
                message.path = if commit.action == "adoptCapture" {
                    Some(commit.path.context("Missing completed file path")?)
                } else {
                    None
                };
                if commit.url.is_some() {
                    message.url = commit.url;
                }
                if commit.name.is_some() {
                    message.name = commit.name;
                }
                if commit.mime.is_some() {
                    message.mime = commit.mime;
                }
                if commit.size.is_some() {
                    message.size = commit.size;
                }
                if message.path.is_some() {
                    message.size = Some(commit.size.context("Missing completed file size")?);
                }
                return strata_app::browser_capture::capture_confirmed(&runtime, &message);
            }
            if message.action == "ping" {
                let reply = runtime
                    .block_on(async {
                        tokio::time::timeout(
                            std::time::Duration::from_millis(750),
                            ipc::call(&state_dir(), Command::List),
                        )
                        .await
                    })
                    .ok()
                    .and_then(Result::ok)
                    .filter(|reply| reply.error.is_none());
                let active = reply
                    .as_ref()
                    .map(|reply| {
                        reply
                            .jobs
                            .iter()
                            .filter(|job| {
                                matches!(
                                    job.status,
                                    strata_core::Status::Downloading | strata_core::Status::Queued
                                )
                            })
                            .count()
                    })
                    .unwrap_or(0);
                return Ok(
                    serde_json::json!({"ok":true,"running":reply.is_some(),"active":active,"bridgeVersion":env!("CARGO_PKG_VERSION")}),
                );
            }
            let command = match message.action.as_str() {
                "add" => Command::Add {
                    url: message.url.ok_or_else(|| anyhow::anyhow!("Missing URL"))?,
                    directory: download_dir(),
                    name: None,
                },
                "cancel" => Command::Cancel {
                    id: message.id.context("Missing handoff ID")?,
                },
                _ => anyhow::bail!("Unknown action"),
            };
            let reply = runtime.block_on(ipc::call(&state_dir(), command))?;
            let reply = strata_app::check_reply(reply)?;
            let job = reply
                .accepted_id
                .as_ref()
                .and_then(|id| reply.jobs.iter().find(|job| &job.id == id));
            Ok(
                serde_json::json!({"ok":true,"active":reply.jobs.iter().filter(|job|matches!(job.status,strata_core::Status::Downloading|strata_core::Status::Queued)).count(),"download":job.map(|job|serde_json::json!({"id":job.id,"filename":job.destination.file_name().unwrap_or_default().to_string_lossy(),"status":job.status}))}),
            )
        })();
        let response = match result {
            Ok(value) => value,
            Err(error) => serde_json::json!({"ok":false,"error":format!("{error:#}")}),
        };
        native::write_message(&mut output, &response)?;
    }
    Ok(())
}
#[cfg(unix)]
fn registration(args: Vec<String>) -> Result<()> {
    use clap::Parser;
    use strata_app::browser_registration::{Browser, Paths, register, unregister};
    #[derive(Parser)]
    struct Args {
        #[arg(long, conflicts_with = "unregister")]
        register: bool,
        #[arg(long, conflicts_with = "register")]
        unregister: bool,
        #[arg(long)]
        chrome_id: Option<String>,
        #[arg(long)]
        chromium_id: Option<String>,
        #[arg(long)]
        edge_id: Option<String>,
        #[arg(long)]
        firefox: bool,
    }
    let args = Args::parse_from(std::iter::once("stratafetch-browser-host".to_owned()).chain(args));
    let paths = Paths::current()?;
    let binary = std::env::current_exe()?;
    if args.unregister {
        unregister(&paths, &binary)?;
        println!("Browser registration removed; downloads and history preserved.");
    } else {
        let mut browsers = std::collections::BTreeMap::new();
        for (browser, id) in [
            (Browser::Chrome, args.chrome_id),
            (Browser::Chromium, args.chromium_id),
            (Browser::Edge, args.edge_id),
        ] {
            if let Some(id) = id {
                browsers.insert(browser, id);
            }
        }
        if args.firefox {
            browsers.insert(Browser::Firefox, "stratafetch@local".into());
        }
        register(&paths, &binary, browsers)?;
        println!(
            "Browser registration saved for this user. Keep the native host at {} and start the desktop or CLI service.",
            binary.display()
        );
    }
    Ok(())
}
#[cfg(not(unix))]
fn registration(_args: Vec<String>) -> Result<()> {
    anyhow::bail!("On Windows, use packaging/register-browser-host.ps1")
}
fn main() {
    if let Err(error) = run() {
        eprintln!("StrataFetch browser host: {error:#}");
        std::process::exit(1);
    }
}
