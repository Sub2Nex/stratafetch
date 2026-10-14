use clap::{Parser, Subcommand};
use std::path::PathBuf;
use strata_app::{Command, check_reply, download_dir, execute, ipc, shutdown, state_dir};
use strata_core::{Manager, Options, Settings, Status};
#[derive(Parser)]
#[command(
    name = "stratafetch",
    version,
    about = "StrataFetch — local-first HTTP download manager"
)]
struct Args {
    #[arg(long, global = true)]
    state: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    /// Fail unless a desktop or service instance is already listening.
    #[arg(long, global = true)]
    require_service: bool,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    /// Add or remove this installation's per-user Linux desktop menu entry.
    #[cfg(target_os = "linux")]
    InstallDesktop {
        #[arg(long)]
        uninstall: bool,
    },
    Export {
        path: PathBuf,
    },
    Import {
        path: PathBuf,
        #[arg(long)]
        directory: PathBuf,
    },
    /// Persist engine limits. Rate is aggregate bytes/second; zero is unlimited.
    Settings {
        #[arg(long, default_value_t = 4)]
        connections: usize,
        #[arg(long, default_value_t = 3)]
        concurrent_jobs: usize,
        #[arg(long, default_value_t = 0)]
        global_bytes_per_second: u64,
        #[arg(long)]
        organize_by_extension: bool,
    },
    /// Queue an inactive job for a Unix UTC start time (desktop/service must stay running).
    Schedule {
        id: String,
        unix_time: u64,
        #[arg(long, default_value = "Other")]
        category: String,
    },
    Add {
        url: String,
        #[arg(short, long)]
        directory: Option<PathBuf>,
        #[arg(long)]
        name: Option<String>,
    },
    List,
    /// Re-read a completed file and compare a supplied or recorded SHA-256.
    Verify {
        id: String,
        sha256: Option<String>,
    },
    Status {
        id: String,
    },
    Pause {
        id: String,
    },
    Resume {
        id: String,
    },
    Cancel {
        id: String,
    },
    Restart {
        id: String,
    },
    Remove {
        id: String,
    },
    Limit {
        id: String,
        bytes_per_second: u64,
    },
    /// Keep an engine running for CLI/browser commands.
    Serve,
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    let directory = args.state.unwrap_or_else(state_dir);
    let filter = match &args.command {
        Action::Status { id } => Some(id.clone()),
        _ => None,
    };
    let serve = matches!(args.command, Action::Serve);
    let check_completion = matches!(
        args.command,
        Action::Add { .. } | Action::Resume { .. } | Action::Restart { .. }
    );
    let command = match args.command {
        #[cfg(target_os = "linux")]
        Action::InstallDesktop { uninstall } => {
            return strata_app::linux_desktop::current(uninstall);
        }
        Action::Export { path } => Command::ExportHistory { path },
        Action::Import { path, directory } => Command::ImportHistory { path, directory },
        Action::Settings {
            connections,
            concurrent_jobs,
            global_bytes_per_second,
            organize_by_extension,
        } => Command::Settings {
            settings: Settings {
                connections,
                concurrent_jobs,
                global_bytes_per_second,
                organize_by_extension,
            },
        },
        Action::Schedule {
            id,
            unix_time,
            category,
        } => Command::Schedule {
            id,
            category,
            scheduled_at: Some(unix_time),
        },
        Action::Add {
            url,
            directory,
            name,
        } => Command::Add {
            url,
            directory: directory.unwrap_or_else(download_dir),
            name,
        },
        Action::List | Action::Status { .. } | Action::Serve => Command::List,
        Action::Verify { id, sha256 } => Command::Verify { id, sha256 },
        Action::Pause { id } => Command::Pause { id },
        Action::Resume { id } => Command::Start { id },
        Action::Cancel { id } => Command::Cancel { id },
        Action::Restart { id } => Command::Restart { id },
        Action::Remove { id } => Command::Remove { id },
        Action::Limit {
            id,
            bytes_per_second,
        } => Command::Limit {
            id,
            bytes_per_second,
        },
    };
    let mut local = None;
    let mut reply = if !serve {
        match ipc::call(&directory, command.clone()).await {
            Ok(reply) => reply,
            Err(error) => {
                if args.require_service {
                    return Err(error.context("No running StrataFetch service is reachable"));
                }
                let manager = Manager::open(&directory, Options::default())?;
                let reply = execute(&manager, command);
                local = Some(manager);
                reply
            }
        }
    } else {
        let manager = Manager::open(&directory, Options::default())?;
        let reply = execute(&manager, Command::List);
        local = Some(manager);
        reply
    };
    reply = check_reply(reply)?;
    if let Some(manager) = local {
        let service = tokio::spawn({
            let directory = directory.clone();
            let manager = manager.clone();
            async move { ipc::serve(&directory, manager).await }
        });
        if serve {
            tokio::signal::ctrl_c().await?;
        } else {
            loop {
                if !manager.is_running() {
                    break;
                }
                tokio::select! {_=tokio::signal::ctrl_c()=>break,_=tokio::time::sleep(std::time::Duration::from_millis(100))=>()}
            }
        }
        shutdown(&manager).await;
        reply.jobs = manager.list();
        service.abort();
    }
    if let Some(id) = filter {
        reply.jobs.retain(|job| job.id == id);
        anyhow::ensure!(!reply.jobs.is_empty(), "Unknown download ID");
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&reply)?);
    } else {
        println!(
            "Engine: {} connections, {} simultaneous, {} B/s aggregate (0 = unlimited)",
            reply.settings.connections,
            reply.settings.concurrent_jobs,
            reply.settings.global_bytes_per_second
        );
        for job in &reply.jobs {
            println!(
                "{}  {:?}  {} / {}  {}",
                job.id,
                job.status,
                job.downloaded,
                job.size
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "?".into()),
                job.destination.display()
            );
            if let Some(error) = &job.error {
                eprintln!("  {error}");
            }
        }
    }
    anyhow::ensure!(
        !check_completion
            || !reply
                .jobs
                .iter()
                .any(|j| Some(&j.id) == reply.accepted_id.as_ref() && j.status == Status::Failed),
        "The requested download failed"
    );
    Ok(())
}
