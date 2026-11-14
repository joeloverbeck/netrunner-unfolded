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

fn run13(path: &std::path::Path, steps: &str) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_netrunner-driver"))
        .args([
            "--engineering-turns",
            "run13",
            "--view",
            "runner",
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
fn real_process_restart_preserves_run_payments_and_the_new_hq_breach_boundary() {
    let directory = tempfile::tempdir().unwrap();
    let uninterrupted = run13(&directory.path().join("whole.json"), "100");
    assert_eq!(uninterrupted["runner_credits"], 4);
    assert_eq!(uninterrupted["corp_credits"], 0);
    assert_eq!(uninterrupted["clicks"][1], 3);
    assert_eq!(uninterrupted["run"]["hosted_credits"], 3);
    assert_eq!(uninterrupted["run_result"], serde_json::Value::Null);
    for steps in ["13", "25", "37", "100"] {
        let path = directory.path().join(format!("resume-{steps}.json"));
        let pending = run13(&path, steps);
        assert_eq!(pending, run13(&path, "0"));
        assert_eq!(run13(&path, "100"), uninterrupted);
    }
}

fn hq14(path: &std::path::Path, stop: Option<&str>, steps: &str, seed: &str) -> serde_json::Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_netrunner-driver"));
    command
        .args([
            "--engineering-turns",
            "hq14",
            "--view",
            "runner",
            "--seed",
            seed,
            "--stop-after",
            steps,
            "--save",
        ])
        .arg(path);
    if let Some(phase) = stop {
        command.args(["--stop-at", phase]);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn hostile_hq_case_restarts_at_manegarm_and_preserves_all_nine_step_outcomes() {
    let dir = tempfile::tempdir().unwrap();
    let saved = dir.path().join("saved.json");
    let pending = hq14(&saved, Some("Manegarm"), "1000", "42");
    assert_eq!(pending["runner_credits"], 4);
    assert_eq!(pending["run"]["hosted_credits"], 3);
    assert_eq!(pending["clicks"][1], 3);
    assert_eq!(pending, hq14(&saved, None, "0", "999"));
    let resumed = hq14(&saved, None, "1000", "999");
    assert_eq!(
        resumed,
        hq14(&dir.path().join("whole.json"), None, "1000", "42")
    );
    assert_eq!(resumed["runner_credits"], 2);
    assert_eq!(resumed["corp_credits"], 8);
    assert_eq!(resumed["agenda_points"], serde_json::json!([2, 2]));
    assert_eq!(resumed["turn_number"], 4);
    for phase in [
        "Breach HQ",
        "Accessing Manegarm",
        "Choose pool / Overclock payment",
        "Accessing Offworld",
    ] {
        let path = dir
            .path()
            .join(format!("nested-{}.json", phase.replace('/', "-")));
        let a = hq14(&path, Some(phase), "1000", "42");
        assert_eq!(a, hq14(&path, None, "0", "999"));
        assert_eq!(resumed, hq14(&path, None, "1000", "999"));
    }
}

#[test]
fn ordinary_native_opponent_responds_after_the_human_finishes_hq_access() {
    use netrunner_unfolded::{
        game::{Action, Player},
        play::{Play, SaveOptions},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.json");
    hq14(&path, Some("Manegarm"), "1000", "42");
    let mut play = Play::load_turns(
        SaveOptions {
            path,
            recover_from: None,
        },
        Player::Runner,
    );
    play.admit();
    for action in [
        Action::SpendManegarmClicks,
        Action::AccessRoot(0),
        Action::TrashAccessed,
        Action::PayRunCredits(3),
        Action::AccessHq,
        Action::FinishAccess,
        Action::AccessHq,
        Action::StealAgenda,
    ] {
        let d = play.observation().unwrap().decision.unwrap();
        play.choose(d.revision, action).unwrap();
    }
    for _ in 0..40 {
        let v = play.observation().unwrap();
        if v.turn_number == 4 && v.phase == "Action phase" {
            assert_eq!(v.runner_credits, 5);
            assert_eq!(v.corp_credits, 11);
            assert_eq!(v.agenda_points, [2, 2]);
            assert!(v.history.iter().any(|s| s == "Corp plays Hedge Fund."));
            return;
        }
        if let Some(d) = v.decision {
            let action = if d.choices.contains(&Action::GainCredit) {
                Action::GainCredit
            } else {
                Action::PassPriority
            };
            play.choose(d.revision, action).unwrap();
        } else {
            play.respond().unwrap();
        }
    }
    panic!("native opponent did not complete its turn");
}
