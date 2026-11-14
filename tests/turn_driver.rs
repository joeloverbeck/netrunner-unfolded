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
fn central15(
    path: &std::path::Path,
    case: &str,
    stop: Option<&str>,
    steps: &str,
    viewer: &str,
) -> serde_json::Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_netrunner-driver"));
    command
        .args([
            "--engineering-turns",
            case,
            "--view",
            viewer,
            "--stop-after",
            steps,
            "--save",
        ])
        .arg(path);
    if let Some(stop) = stop {
        command.args(["--stop-at", stop]);
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
fn central_and_agenda_decisions_resume_across_real_process_exit() {
    let dir = tempfile::tempdir().unwrap();
    for (case, phase, viewer) in [
        ("central15", "Send a Message", "corp"),
        ("winning15", "Accessing Send a Message", "runner"),
        ("hub15", "Superconducting Hub", "corp"),
    ] {
        let path = dir.path().join(format!("{case}.json"));
        let pending = central15(&path, case, Some(phase), "100", viewer);
        assert!(pending["phase"].as_str().unwrap().starts_with(phase));
        assert_eq!(pending, central15(&path, case, None, "0", viewer));
        let final_view = central15(
            &path,
            case,
            None,
            if case == "hub15" { "1" } else { "100" },
            viewer,
        );
        if case == "hub15" {
            assert_eq!(final_view["hq_count"], 3);
        } else {
            let whole = central15(
                &dir.path().join(format!("whole-{case}.json")),
                case,
                None,
                "100",
                viewer,
            );
            assert_eq!(whole, final_view);
            assert_eq!(
                final_view["agenda_points"][1],
                if case == "winning15" { 6 } else { 5 }
            );
        }
    }
}

#[test]
fn corp_handoff_choices_settle_through_the_real_native_opponent_path() {
    use netrunner_unfolded::{
        game::{Action, Player},
        play::{Play, SaveOptions},
    };
    let dir = tempfile::tempdir().unwrap();
    for (case, phase) in [
        ("central15", "Send a Message"),
        ("hub15", "Superconducting Hub"),
    ] {
        let path = dir.path().join(format!("{case}.json"));
        let initial = central15(&path, case, Some(phase), "100", "corp");
        if case == "hub15" {
            assert_eq!(initial["hq_count"], 1);
            assert_eq!(initial["rd_count"], 2);
        }
        let mut play = Play::load_turns(
            SaveOptions {
                path,
                recover_from: None,
            },
            Player::Corp,
        );
        play.admit();
        let d = play.observation().unwrap().decision.unwrap();
        let action = if case == "hub15" {
            Action::DrawAgendaCards
        } else {
            d.choices
                .into_iter()
                .find(|a| matches!(a, Action::RezIceFree(_)))
                .unwrap()
        };
        play.choose(d.revision, action).unwrap();
        for _ in 0..80 {
            let v = play.observation().unwrap();
            if v.active_player == Some(Player::Corp) && v.phase == "Action phase" {
                break;
            }
            play.respond().unwrap();
        }
        let v = play.observation().unwrap();
        assert_eq!(v.phase, "Action phase");
        assert_eq!(v.active_player, Some(Player::Corp));
        if case == "hub15" {
            assert_eq!((v.hq_count, v.rd_count, v.agenda_points), (3, 0, [1, 0]));
        } else {
            assert_eq!(
                (
                    v.runner_credits,
                    v.corp_credits,
                    v.agenda_points,
                    v.turn_number,
                    v.hq_count,
                    v.rd_count
                ),
                (11, 0, [0, 5], 3, 1, 30)
            );
            assert!(v.servers[0].ice[0].rezzed);
        }
    }
}
fn damage16(
    path: &std::path::Path,
    case: &str,
    phase: Option<&str>,
    steps: &str,
    seed: &str,
) -> serde_json::Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_netrunner-driver"));
    command
        .args([
            "--engineering-turns",
            case,
            "--view",
            "runner",
            "--stop-after",
            steps,
            "--seed",
            seed,
            "--save",
        ])
        .arg(path);
    if let Some(phase) = phase {
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
fn damage_choices_resume_in_separate_processes_without_repeating_damage() {
    let dir = tempfile::tempdir().unwrap();
    for (case, phase, grip, heap) in [
        ("karuna16", "Karunā", 3, 2),
        ("damage16", "Accessing Urtica", 0, 4),
    ] {
        let path = dir.path().join(format!("{case}.json"));
        let pending = damage16(&path, case, Some(phase), "100", "42");
        assert_eq!(pending["grip_count"], grip);
        assert_eq!(pending["heap"].as_array().unwrap().len(), heap);
        assert_eq!(pending, damage16(&path, case, None, "0", "999"));
        let resumed = damage16(&path, case, None, "100", "999");
        assert_eq!(
            resumed,
            damage16(
                &dir.path().join(format!("{case}-whole.json")),
                case,
                None,
                "100",
                "42"
            )
        );
        if case == "karuna16" {
            assert_eq!(resumed["match_winner"], "Corp");
            assert_eq!(resumed["grip_count"], 1);
            assert_eq!(resumed["heap"].as_array().unwrap().len(), 4);
        } else {
            assert_eq!(resumed["match_winner"], serde_json::Value::Null);
            assert_eq!(resumed["run_result"], "SuccessfulAccess");
            assert_eq!(resumed["runner_credits"], 7);
            assert_eq!(resumed["corp_credits"], 1);
        }
    }
}
#[test]
fn actual_play_responses_settle_karuna_and_trap_choices_with_exact_counts() {
    use netrunner_unfolded::{
        game::{Action, Player},
        play::{Play, SaveOptions},
    };
    fn settle(play: &mut Play) {
        for _ in 0..30 {
            let v = play.observation().unwrap();
            if v.decision.is_some() || v.match_winner.is_some() {
                return;
            }
            play.respond().unwrap();
        }
        panic!("native opponent did not settle");
    }
    fn act(play: &mut Play, action: Action) {
        let d = play.observation().unwrap().decision.unwrap();
        assert!(
            d.choices.contains(&action),
            "{action:?} not in {:?}",
            d.choices
        );
        play.choose(d.revision, action).unwrap();
        settle(play);
    }
    let dir = tempfile::tempdir().unwrap();
    for jack_out in [true, false] {
        let path = dir.path().join(format!("karuna-{jack_out}.json"));
        damage16(&path, "karuna16", Some("Karunā"), "100", "42");
        let mut play = Play::load_turns(
            SaveOptions {
                path,
                recover_from: None,
            },
            Player::Runner,
        );
        play.admit();
        act(
            &mut play,
            if jack_out {
                Action::JackOut
            } else {
                Action::Continue
            },
        );
        if jack_out {
            let v = play.observation().unwrap();
            assert_eq!(
                (
                    v.hand.len(),
                    v.heap.len(),
                    v.runner_credits,
                    v.corp_credits,
                    v.clicks[1]
                ),
                (3, 2, 8, 0, 3)
            );
            assert!(v.run.is_none());
            assert_eq!(v.phase, "Action phase");
        } else {
            assert_eq!(
                play.observation().unwrap().phase,
                "Movement · continue or jack out"
            );
            act(&mut play, Action::Continue);
            assert_eq!(
                play.observation().unwrap().phase,
                "Breach remote · choose a root card"
            );
            act(&mut play, Action::AccessRoot(0));
            let v = play.observation().unwrap();
            assert_eq!(
                (v.hand.len(), v.heap.len(), v.runner_credits, v.corp_credits),
                (1, 4, 8, 0)
            );
            assert_eq!(v.match_winner, Some(Player::Corp));
        }
    }
    let path = dir.path().join("trap.json");
    damage16(&path, "damage16", Some("Breach remote"), "100", "42");
    let mut play = Play::load_turns(
        SaveOptions {
            path,
            recover_from: None,
        },
        Player::Runner,
    );
    play.admit();
    act(&mut play, Action::AccessRoot(0));
    assert_eq!(play.observation().unwrap().hand.len(), 0);
    act(&mut play, Action::TrashAccessed);
    act(&mut play, Action::PayRunCredits(2));
    let v = play.observation().unwrap();
    assert_eq!(
        (
            v.hand.len(),
            v.heap.len(),
            v.runner_credits,
            v.corp_credits,
            v.clicks[1]
        ),
        (0, 5, 7, 1, 3)
    );
    assert_eq!(v.phase, "Action phase");
    assert!(v.run.is_none());
}
