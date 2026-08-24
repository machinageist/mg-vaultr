use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn command(home: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mg-vault"));
    command
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_CACHE_HOME", home.path().join("cache"));
    command
}

#[test]
fn json_registry_and_note_flow_is_versioned() {
    let home = tempfile::tempdir().unwrap();
    let vault = home.path().join("vault");
    std::fs::create_dir(&vault).unwrap();

    assert!(
        command(&home)
            .args(["vault", "register", "test", vault.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        command(&home)
            .args(["vault", "select", "test"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        command(&home)
            .args(["--no-input", "note", "create", "a.md", "--body", "hello"])
            .status()
            .unwrap()
            .success()
    );

    let output = command(&home)
        .args(["--json", "--no-color", "note", "read", "a.md"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["content"], "hello");
}
