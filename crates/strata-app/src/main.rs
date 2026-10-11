use clap::{Parser,Subcommand};
use std::path::PathBuf;
use strata_app::{Command,check_reply,download_dir,execute,ipc,shutdown,state_dir};
use strata_core::{Manager,Options,Status};
#[derive(Parser)]
#[command(version,about="StrataFetch — local-first HTTP download manager")]
struct Args {
    #[arg(long,global=true)] state:Option<PathBuf>,
    #[arg(long,global=true)] json:bool,
    #[command(subcommand)] command:Action,
}
#[derive(Subcommand)]
enum Action {
    Add {url:String,#[arg(short,long)] directory:Option<PathBuf>,#[arg(long)] name:Option<String>},
    List,Status {id:String},Pause {id:String},Resume {id:String},Cancel {id:String},Restart {id:String},
    Remove {id:String},Limit {id:String,bytes_per_second:u64},
    /// Keep an engine running for CLI/browser commands.
    Serve,
}
#[tokio::main]
async fn main()->anyhow::Result<()> {
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();
    let args=Args::parse();let directory=args.state.unwrap_or_else(state_dir);
    let filter=match &args.command {Action::Status {id}=>Some(id.clone()),_=>None};
    let serve=matches!(args.command,Action::Serve);
    let command=match args.command {
        Action::Add {url,directory,name}=>Command::Add {url,directory:directory.unwrap_or_else(download_dir),name},
        Action::List|Action::Status {..}|Action::Serve=>Command::List,
        Action::Pause {id}=>Command::Pause {id},Action::Resume {id}=>Command::Start {id},Action::Cancel {id}=>Command::Cancel {id},
        Action::Restart {id}=>Command::Restart {id},Action::Remove {id}=>Command::Remove {id},Action::Limit {id,bytes_per_second}=>Command::Configure {id,priority:0,bytes_per_second},
    };
    let mut local=None;
    let mut reply=if !serve {match ipc::call(&directory,command.clone()).await {Ok(reply)=>reply,Err(_)=>{let manager=Manager::open(&directory,Options::default())?;let reply=execute(&manager,command);local=Some(manager);reply}}}
        else {let manager=Manager::open(&directory,Options::default())?;let reply=execute(&manager,Command::List);local=Some(manager);reply};
    reply=check_reply(reply)?;
    if let Some(manager)=local {
        let service=tokio::spawn({let directory=directory.clone();let manager=manager.clone();async move {ipc::serve(&directory,manager).await}});
        if serve {tokio::signal::ctrl_c().await?;}
        else {loop {if !manager.is_running(){break;}tokio::select! {_=tokio::signal::ctrl_c()=>break,_=tokio::time::sleep(std::time::Duration::from_millis(100))=>()}}}
        shutdown(&manager).await;reply.jobs=manager.list();service.abort();
    }
    if let Some(id)=filter {reply.jobs.retain(|job|job.id==id);anyhow::ensure!(!reply.jobs.is_empty(),"Unknown download ID");}
    if args.json {println!("{}",serde_json::to_string_pretty(&reply)?);} else {for job in &reply.jobs {println!("{}  {:?}  {} / {}  {}",job.id,job.status,job.downloaded,job.size.map(|n|n.to_string()).unwrap_or_else(||"?".into()),job.destination.display());if let Some(error)=&job.error {eprintln!("  {error}");}}}
    anyhow::ensure!(!reply.jobs.iter().any(|j|j.status==Status::Failed),"One or more downloads failed"); Ok(())
}
