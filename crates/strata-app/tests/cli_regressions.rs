use serde_json::Value;
use std::path::Path;
fn cli(state: &Path, args: &[&str]) -> Value {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_stratafetch"))
        .arg("--state")
        .arg(state)
        .arg("--json")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn changing_rate_limit_preserves_queue_priority() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let out = dir.path().join("downloads");
    let history = dir.path().join("history.json");
    std::fs::write(&history, r#"{"version":1,"entries":[{"url":"https://example.org/file","filename":"file","category":"Other","priority":77,"bytes_per_second":0}]}"#).unwrap();
    let added = cli(
        &state,
        &[
            "import",
            history.to_str().unwrap(),
            "--directory",
            out.to_str().unwrap(),
        ],
    );
    let id = added["jobs"][0]["id"].as_str().unwrap();
    let changed = cli(&state, &["limit", id, "12345"]);
    assert_eq!(changed["jobs"][0]["priority"], 77);
    assert_eq!(changed["jobs"][0]["bytes_per_second"], 12345);
}
