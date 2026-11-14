use std::process::Command;

fn run(path: &std::path::Path, case: &str, seed: &str, steps: &str) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_netrunner-driver"))
        .args([
            "--engineering-turns",
            case,
            "--seed",
            seed,
            "--stop-after",
            steps,
            "--save",
        ])
        .arg(path)
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
fn fresh_process_resumes_turns_and_committed_shuffle_without_rerolling() {
    let directory = tempfile::tempdir().unwrap();
    let saved = directory.path().join("saved.json");
    let uninterrupted = directory.path().join("uninterrupted.json");
    let pending = run(&saved, "economy", "42", "7");
    assert_eq!(pending, run(&saved, "economy", "999", "0"));
    assert_eq!(
        run(&saved, "economy", "999", "200"),
        run(&uninterrupted, "economy", "42", "207")
    );
}

#[test]
fn saved_pending_install_discard_and_seamless_choices_survive_process_exit() {
    let directory = tempfile::tempdir().unwrap();
    for (case, steps, phase) in [
        (
            "install",
            "3",
            "Installation · optional trash before paying",
        ),
        ("discard", "4", "Choose cards to discard simultaneously"),
        (
            "scoring",
            "9",
            "Seamless Launch · choose one installed card",
        ),
    ] {
        let path = directory.path().join(format!("{case}.json"));
        let first = run(&path, case, "42", steps);
        assert_eq!(first["phase"], phase);
        assert_eq!(first, run(&path, case, "999", "0"));
    }
}
