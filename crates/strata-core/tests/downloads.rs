use axum::{Router, body::Body, extract::State, http::{HeaderMap, Response, StatusCode}, routing::get};
use strata_core::{Manager, Options, Status};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tempfile::TempDir;

#[derive(Clone)]
struct Server { data: Arc<Vec<u8>>, mode: u8, version: Arc<AtomicUsize>, ranges: Arc<AtomicUsize> }
async fn handler(State(server): State<Server>, headers: HeaderMap) -> Response<Body> {
    let etag=format!("\"v{}\"",server.version.load(Ordering::SeqCst));
    if server.mode==6 { return Response::builder().status(302).header("Location","/file").body(Body::empty()).unwrap(); }
    if let Some(range)=headers.get("range").and_then(|v|v.to_str().ok()) {
        if server.mode != 1 && server.mode != 5 {
            let (a,b)=range.strip_prefix("bytes=").unwrap().split_once('-').unwrap(); let a:usize=a.parse().unwrap();let b:usize=b.parse().unwrap();
            server.ranges.fetch_add(1,Ordering::SeqCst);
            if a>=server.data.len() {return Response::builder().status(416).body(Body::empty()).unwrap();}
            let b=b.min(server.data.len()-1);
            let advertised=if server.mode==2 {a+1} else {a};
            let body=if server.mode==3 && b>0 {server.data[a..b].to_vec()} else {server.data[a..=b].to_vec()};
            if server.mode==4 && b>0 {return Response::builder().status(200).header("ETag",&etag).body(Body::from(body)).unwrap();}
            return Response::builder().status(206).header("ETag",&etag).header("Content-Range",format!("bytes {advertised}-{b}/{}",server.data.len())).body(Body::from(body)).unwrap();
        }
    }
    let mut response=Response::builder().status(StatusCode::OK).header("ETag",etag);
    if server.mode==5 {
        response=response.header("Transfer-Encoding","chunked");
        let chunks=server.data.chunks(8192).map(|c|Ok::<_,std::io::Error>(c.to_vec())).collect::<Vec<_>>();
        response.body(Body::from_stream(futures_util::stream::iter(chunks))).unwrap()
    } else {response.body(Body::from(server.data.as_ref().clone())).unwrap()}
}
async fn server(mode:u8,size:usize)->(String,Server,tokio::task::JoinHandle<()>) {
    let state=Server {data:Arc::new((0..size).map(|i|(i%251) as u8).collect()),mode,version:Arc::new(AtomicUsize::new(1)),ranges:Arc::new(AtomicUsize::new(0))};
    let app=Router::new().route("/file",get(handler)).with_state(state.clone());
    let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();
    let task=tokio::spawn(async move {axum::serve(listener,app).await.unwrap();});
    (format!("http://{address}/file"),state,task)
}
async fn wait(manager:&Manager,id:&str)->strata_core::Job {
    tokio::time::timeout(std::time::Duration::from_secs(30),async {loop {let job=manager.get(id).unwrap(); if !matches!(job.status,Status::Queued|Status::Downloading) && !manager.is_running() {return job;} tokio::time::sleep(std::time::Duration::from_millis(20)).await;}}).await.unwrap()
}
#[tokio::test]
async fn downloads_and_safe_fallbacks() {
    for mode in [0,1,5] {
        let (url,state,task)=server(mode,1_100_000).await;let dir=TempDir::new().unwrap();
        let manager=Manager::open(&dir.path().join("state"),Options::default()).unwrap();let job=manager.add(&url,&dir.path().join("out"),Some("test.bin")).unwrap();
        manager.start(&job.id).unwrap();let job=wait(&manager,&job.id).await;
        assert_eq!(job.status,Status::Completed,"{:?}",job.error); assert_eq!(std::fs::read(&job.destination).unwrap(),*state.data);assert!(job.sha256.is_some());
        if mode==0 {assert_eq!(job.segments.len(),4);assert!(state.ranges.load(Ordering::SeqCst)>=5);}
        task.abort();
    }
}
#[tokio::test]
async fn rejects_inconsistent_ranges() {
    for mode in [2,3,4] {
        let (url,_,task)=server(mode,600_000).await;let dir=TempDir::new().unwrap();let manager=Manager::open(&dir.path().join("state"),Options {retries:0,..Default::default()}).unwrap();
        let job=manager.add(&url,dir.path(),None).unwrap();manager.start(&job.id).unwrap();let job=wait(&manager,&job.id).await;
        assert_eq!(job.status,Status::Failed);assert!(!job.destination.exists()); task.abort();
    }
}
#[tokio::test]
async fn zero_bytes_and_collision_protection() {
    let (url,_,task)=server(1,0).await;let dir=TempDir::new().unwrap();let manager=Manager::open(&dir.path().join("state"),Options::default()).unwrap();
    std::fs::write(dir.path().join("file"),b"keep").unwrap();let job=manager.add(&url,dir.path(),None).unwrap();assert_ne!(job.destination,dir.path().join("file"));
    manager.start(&job.id).unwrap();assert_eq!(wait(&manager,&job.id).await.status,Status::Completed);assert_eq!(std::fs::read(dir.path().join("file")).unwrap(),b"keep"); task.abort();
}
#[tokio::test]
async fn pause_resume_changed_validator_and_recovery() {
    let (url,state,task)=server(0,2_000_000).await;let dir=TempDir::new().unwrap();let state_dir=dir.path().join("state");
    let manager=Manager::open(&state_dir,Options::default()).unwrap();let mut job=manager.add(&url,dir.path(),None).unwrap();job.bytes_per_second=100_000;
    // Configure persistent limits through the public setter.
    manager.configure(&job.id,0,100_000).unwrap();manager.start(&job.id).unwrap();tokio::time::sleep(std::time::Duration::from_millis(400)).await;manager.pause(&job.id).unwrap();
    let paused=wait(&manager,&job.id).await;assert!(paused.downloaded>0);assert_eq!(paused.status,Status::Paused);drop(manager);
    let manager=Manager::open(&state_dir,Options::default()).unwrap();assert_eq!(manager.get(&job.id).unwrap().downloaded,paused.downloaded);
    state.version.store(2,Ordering::SeqCst);manager.start(&job.id).unwrap();let failed=wait(&manager,&job.id).await;assert_eq!(failed.status,Status::Failed);assert!(!failed.destination.exists());
    manager.configure(&job.id,0,0).unwrap();manager.restart(&job.id).unwrap();assert_eq!(wait(&manager,&job.id).await.status,Status::Completed);task.abort();
}
#[test]
fn metadata_and_lock_security() {
    use strata_core::{sanitize_filename,network::parse_range};
    assert_eq!(sanitize_filename("../../CON"),"_.._CON");assert_eq!(sanitize_filename("CON.txt"),"download_CON.txt");
    assert!(parse_range("bytes 0-10/10").is_err());assert!(parse_range("bytes 4294967296-4294967297/8589934592").is_ok());
    let dir=TempDir::new().unwrap();let manager=Manager::open(dir.path(),Options::default()).unwrap();assert!(Manager::open(dir.path(),Options::default()).is_err());
    assert!(manager.add("file:///etc/passwd",dir.path(),None).is_err());assert!(manager.add("https://user:pass@example.org/file",dir.path(),None).is_err());
}
