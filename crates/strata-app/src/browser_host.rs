//! Official browser native-messaging framing. This bridge requires a running manager.
use anyhow::{Result, ensure};
use strata_app::{Command, download_dir, ipc, native, state_dir};
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
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
    while let Some(message) = native::read_message(&mut input)? {
        let result: Result<_> = (|| {
            let command = match message.action.as_str() {
                "ping" => Command::List,
                "add" => Command::Add {
                    url: message.url.ok_or_else(|| anyhow::anyhow!("Missing URL"))?,
                    directory: download_dir(),
                    name: None,
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
