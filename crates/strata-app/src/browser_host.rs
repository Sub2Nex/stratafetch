//! Official browser native-messaging framing. This bridge requires a running manager.
use anyhow::{Result, ensure};
use strata_app::{Command, download_dir, ipc, native, state_dir};
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
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
            strata_app::check_reply(reply)?;
            Ok(serde_json::json!({"ok":true}))
        })();
        let response = match result {
            Ok(value) => value,
            Err(error) => serde_json::json!({"ok":false,"error":format!("{error:#}")}),
        };
        native::write_message(&mut output, &response)?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("StrataFetch browser host: {error:#}");
        std::process::exit(1);
    }
}
