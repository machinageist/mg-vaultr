use std::fs;
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

fn register(home: &TempDir, vault: &std::path::Path) {
    register_named(home, "test", vault);
}

fn register_named(home: &TempDir, name: &str, vault: &std::path::Path) {
    assert!(
        command(home)
            .args(["vault", "register", name, vault.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn index_rebuild_and_search_are_json_sorted_and_diagnostic() {
    let home = tempfile::tempdir().unwrap();
    let vault = home.path().join("vault");
    fs::create_dir(&vault).unwrap();
    register(&home, &vault);
    fs::write(vault.join("z.md"), "# Zed\nneedle\n").unwrap();
    fs::create_dir(vault.join("nested")).unwrap();
    fs::write(
        vault.join("nested/a.md"),
        "---\ntitle: Alpha\n---\nneedle\n",
    )
    .unwrap();
    fs::write(vault.join("bad.md"), [0xff, 0xfe]).unwrap();

    let status = command(&home)
        .args(["--json", "index", "rebuild"])
        .output()
        .unwrap();
    assert!(status.status.success());
    let status: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["data"]["status"], "degraded");
    assert_eq!(status["data"]["generation"], 1);
    assert_eq!(status["data"]["note_count"], 2);
    assert_eq!(status["data"]["degraded"], true);
    assert_eq!(status["data"]["freshness"], "rebuild_snapshot");
    assert_eq!(status["data"]["persistence"], "none");
    assert_eq!(status["data"]["derived_from"], "authoritative_vault_files");
    assert_eq!(status["data"]["diagnostics"][0]["path"], "bad.md");

    let search = command(&home)
        .args(["--json", "search", "NEEDLE"])
        .output()
        .unwrap();
    assert!(search.status.success());
    let search: Value = serde_json::from_slice(&search.stdout).unwrap();
    assert_eq!(search["data"]["results"][0]["path"], "nested/a.md");
    assert_eq!(search["data"]["results"][1]["path"], "z.md");
    assert_eq!(search["data"]["status"], "degraded");

    let human = command(&home).args(["search", "needle"]).output().unwrap();
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("source=authoritative_vault_files"));
    assert!(human.contains("freshness=rebuild_snapshot"));
    assert!(human.contains("persistence=none"));
    assert!(human.contains("degraded: bad.md:"));
}

#[test]
fn index_commands_report_no_selected_vault() {
    let home = tempfile::tempdir().unwrap();
    let output = command(&home)
        .args(["--json", "index", "status"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["ok"], false);
    assert_eq!(error["error"]["code"], "no_vault_selected");
}

#[test]
fn index_search_can_target_a_named_vault() {
    let home = tempfile::tempdir().unwrap();
    let first = home.path().join("first");
    let second = home.path().join("second");
    fs::create_dir(&first).unwrap();
    fs::create_dir(&second).unwrap();
    register_named(&home, "first", &first);
    register_named(&home, "second", &second);
    fs::write(first.join("first.md"), "first-only\n").unwrap();
    fs::write(second.join("second.md"), "second-only needle\n").unwrap();

    let output = command(&home)
        .args(["--json", "--vault", "second", "search", "needle"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["data"]["results"][0]["path"], "second.md");
    assert_eq!(value["data"]["results"].as_array().unwrap().len(), 1);
}
