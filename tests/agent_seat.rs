#![cfg(feature = "tabletop")]

use std::{fs, process::Command};

#[test]
fn missing_art_refuses_the_agent_seat_before_reading_or_writing_a_save() {
    let root = tempfile::tempdir().unwrap();
    let save = root.path().join("owner.json");
    fs::write(&save, b"owner's existing save").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_netrunner-tabletop"))
        .args(["--agent-seat", "--assets"])
        .arg(root.path().join("missing"))
        .arg("--save")
        .arg(&save)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["status"], "refused");
    assert_eq!(response["mode"], "seat");
    assert_eq!(fs::read(save).unwrap(), b"owner's existing save");
    assert!(!unfolded_record_replacement::lock_path(&root.path().join("owner.json")).exists());
}

#[test]
fn invalid_agent_options_refuse_without_starting_a_renderer() {
    for arguments in [
        vec!["--agent-seat", "--socket"],
        vec!["--agent-seat", "--socket", "--turns"],
        vec!["--agent-seat", "--turns"],
        vec!["--agent-seat", "--seat", "unknown"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_netrunner-tabletop"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["status"], "refused");
        assert!(response["error"].as_str().unwrap().contains("Usage:"));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_netrunner-tabletop"))
        .args(["--socket", "/tmp/should-not-be-created-netrunner.sock"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("socket requires --agent-seat")
    );
}
