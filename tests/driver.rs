use std::{
    path::Path,
    process::{Command, Output},
};

fn driver(path: &Path, steps: usize, viewer: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_netrunner-driver"))
        .args(["rez", "--save"])
        .arg(path)
        .args(["--stop-after", &steps.to_string(), "--view", viewer])
        .output()
        .unwrap()
}
fn observation(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn fresh_process_resumes_each_suspended_decision_with_lawful_knowledge() {
    let directory = tempfile::tempdir().unwrap();
    for steps in 0..=3 {
        let path = directory.path().join(format!("match-{steps}.json"));
        let corp = observation(driver(&path, steps, "corp"));
        assert_eq!(observation(driver(&path, 0, "corp")), corp);
        assert_eq!(corp["corp_credits"], 3);
        let runner = observation(driver(&path, 0, "runner"));
        assert!(!runner.to_string().contains("Palisade"));
        let final_state = observation(driver(&path, 10, "corp"));
        assert_eq!(final_state["corp_credits"], 0);
        assert_eq!(final_state["run_result"], "EndedByIce");
        let uninterrupted = directory.path().join(format!("whole-{steps}.json"));
        assert_eq!(final_state, observation(driver(&uninterrupted, 10, "corp")));
    }
}

#[test]
fn corrupt_active_record_requires_explicit_recovery_into_a_fresh_path() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("match.json");
    let pending = observation(driver(&source, 1, "corp"));
    observation(driver(&source, 1, "corp"));
    let previous = unfolded_tabletop::record_replacement::previous_path(&source);
    let previous_bytes = std::fs::read(&previous).unwrap();
    std::fs::write(&source, b"interrupted write").unwrap();
    assert!(!driver(&source, 0, "corp").status.success());
    let target = directory.path().join("recovered.json");
    let recover = |target: &Path| {
        Command::new(env!("CARGO_BIN_EXE_netrunner-driver"))
            .arg("--save")
            .arg(target)
            .arg("--recover-from")
            .arg(&source)
            .args(["--stop-after", "0"])
            .output()
            .unwrap()
    };
    assert_eq!(observation(recover(&target)), pending);
    assert!(!recover(&source).status.success());
    assert!(!recover(&previous).status.success());
    assert!(!recover(&target).status.success());
    assert_eq!(std::fs::read(&source).unwrap(), b"interrupted write");
    assert_eq!(std::fs::read(&previous).unwrap(), previous_bytes);
    assert_eq!(
        observation(driver(&target, 10, "corp"))["run_result"],
        "EndedByIce"
    );
}

fn resign(envelope: &mut serde_json::Value) {
    use sha2::{Digest, Sha256};
    let fields =
        ["format", "kind", "engine", "content", "payload"].map(|field| envelope[field].clone());
    envelope["checksum"] =
        format!("{:x}", Sha256::digest(serde_json::to_vec(&fields).unwrap())).into();
}

#[test]
fn unsupported_or_damaged_records_are_refused_without_touching_either_copy() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("match.json");
    observation(driver(&source, 1, "corp"));
    let original = std::fs::read(&source).unwrap();
    let previous = unfolded_tabletop::record_replacement::previous_path(&source);
    let previous_bytes = std::fs::read(&previous).unwrap();
    for (pointer, value) in [
        ("/format", serde_json::json!(22)),
        ("/engine", serde_json::json!("unsupported")),
        ("/content", serde_json::json!("missing content")),
        ("/payload/rules", serde_json::json!("unsupported rules")),
        ("/payload/state/pending", serde_json::Value::Null),
        ("/checksum", serde_json::json!("wrong checksum")),
    ] {
        let mut envelope: serde_json::Value = serde_json::from_slice(&original).unwrap();
        *envelope.pointer_mut(pointer).unwrap() = value;
        if pointer != "/checksum" {
            resign(&mut envelope);
        }
        let damaged = serde_json::to_vec(&envelope).unwrap();
        std::fs::write(&source, &damaged).unwrap();
        assert!(!driver(&source, 0, "corp").status.success(), "{pointer}");
        assert_eq!(std::fs::read(&source).unwrap(), damaged);
        assert_eq!(std::fs::read(&previous).unwrap(), previous_bytes);
    }
    std::fs::write(&source, vec![b'x'; 1024 * 1024 + 1]).unwrap();
    assert!(!driver(&source, 0, "corp").status.success());
    std::fs::remove_file(&source).unwrap();
    std::os::unix::fs::symlink(&previous, &source).unwrap();
    assert!(!driver(&source, 0, "corp").status.success());
    assert_eq!(std::fs::read(&previous).unwrap(), previous_bytes);
}

#[test]
fn exhausted_revision_refuses_restart_instead_of_wrapping() {
    use netrunner_unfolded::play::Play;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    observation(driver(&path, 10, "corp"));
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    envelope["payload"]["state"]["revision"] = u64::MAX.into();
    resign(&mut envelope);
    let bytes = serde_json::to_vec(&envelope).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let mut play = Play::saved(&path);
    play.admit();
    let before = play.observation();
    assert!(play.restart().is_err());
    assert_eq!(play.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn missing_active_record_preserves_its_previous_decision_for_explicit_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("match.json");
    observation(driver(&source, 1, "corp"));
    let previous = unfolded_tabletop::record_replacement::previous_path(&source);
    let previous_bytes = std::fs::read(&previous).unwrap();
    std::fs::remove_file(&source).unwrap();
    assert!(!driver(&source, 0, "corp").status.success());
    assert!(!source.exists());
    assert_eq!(std::fs::read(&previous).unwrap(), previous_bytes);
}

#[cfg(feature = "tabletop")]
#[test]
fn native_startup_with_missing_artwork_never_creates_or_replaces_save_files() {
    let directory = tempfile::tempdir().unwrap();
    let save = directory.path().join("match.json");
    let launch = || {
        Command::new(env!("CARGO_BIN_EXE_netrunner-tabletop"))
            .arg("--save")
            .arg(&save)
            .arg("--assets")
            .arg(directory.path().join("no-artwork"))
            .output()
            .unwrap()
    };
    assert!(!launch().status.success());
    assert!(!save.exists());
    observation(driver(&save, 1, "corp"));
    let bytes = std::fs::read(&save).unwrap();
    assert!(!launch().status.success());
    assert_eq!(std::fs::read(&save).unwrap(), bytes);
}
