use netrunner_unfolded::{game::Action, play::Play};

#[test]
fn suspended_installation_resumes_through_the_native_controller() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    assert!(!path.exists());
    play.admit();
    play.choose(0, Action::InstallPalisade).unwrap();
    let pending = play.observation().unwrap();
    drop(play);
    let mut resumed = Play::saved(&path);
    assert!(resumed.observation().is_none());
    resumed.admit();
    assert_eq!(resumed.observation().unwrap(), pending);
    resumed.choose(1, Action::TrashBran).unwrap();
    resumed.respond().unwrap();
    let revision = resumed.observation().unwrap().decision.unwrap().revision;
    resumed.choose(revision, Action::RezPalisade).unwrap();
    assert_eq!(resumed.observation().unwrap().corp_credits, 0);
}

#[test]
fn failed_opponent_save_keeps_the_pending_decision_and_requires_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    play.admit();
    play.choose(0, Action::InstallPalisade).unwrap();
    play.choose(1, Action::TrashBran).unwrap();
    let before = play.observation();
    let bytes = std::fs::read(&path).unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.path().join("match.json.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(play.respond().unwrap_err().contains("busy"));
    assert_eq!(play.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    assert!(
        play.respond().is_err(),
        "a failed operation must not silently retry"
    );
    let mut resumed = Play::saved(&path);
    resumed.admit();
    resumed.respond().unwrap();
    assert_eq!(
        resumed.observation().unwrap().decision.unwrap().choices,
        vec![Action::RezPalisade, Action::DeclineRez]
    );
}

#[test]
fn another_process_and_a_bad_recovery_record_cannot_be_overwritten() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut first = Play::saved(&path);
    first.admit();
    let mut stale = Play::saved(&path);
    stale.admit();
    let before = stale.observation();
    first.choose(0, Action::InstallPalisade).unwrap();
    let committed = std::fs::read(&path).unwrap();
    assert!(stale.choose(0, Action::DeclineInstall).is_err());
    assert_eq!(stale.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), committed);
    let previous = unfolded_tabletop::record_replacement::previous_path(&path);
    std::fs::write(&previous, b"damaged previous").unwrap();
    let before = first.observation();
    assert!(first.choose(1, Action::TrashBran).is_err());
    assert_eq!(first.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), committed);
    assert_eq!(std::fs::read(&previous).unwrap(), b"damaged previous");
}

#[test]
fn restart_is_persisted_before_install_and_old_choices_remain_stale_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    play.admit();
    play.choose(0, Action::DeclineInstall).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let before = play.observation();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.path().join("match.json.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(play.restart().is_err());
    assert_eq!(play.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    let mut reopened = Play::saved(&path);
    reopened.admit();
    reopened.restart().unwrap();
    drop(reopened);
    let mut reopened = Play::saved(&path);
    reopened.admit();
    let before = reopened.observation();
    assert!(reopened.choose(0, Action::InstallPalisade).is_err());
    assert_eq!(reopened.observation(), before);
    assert_eq!(before.unwrap().decision.unwrap().revision, 2);
}

#[test]
fn artwork_refusal_cannot_create_or_change_a_save() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    play.refuse("required artwork unavailable".into());
    play.admit();
    assert!(play.observation().is_none());
    assert!(!path.exists());
    std::fs::write(&path, b"existing record").unwrap();
    let mut play = Play::saved(&path);
    play.refuse("required artwork unavailable".into());
    play.admit();
    assert_eq!(std::fs::read(&path).unwrap(), b"existing record");
}

#[test]
fn artwork_admission_precedes_game_creation_and_failure_precedes_every_choice() {
    let mut play = Play::default();
    assert!(play.observation().is_none());
    assert!(play.choose(0, Action::InstallPalisade).is_err());
    play.admit();
    let before = play.observation().unwrap();
    play.refuse("required texture disappeared".into());
    assert!(play.choose(0, Action::DeclineInstall).is_err());
    assert!(play.restart().is_err());
    assert_eq!(play.observation().unwrap(), before);
}

#[test]
fn the_native_controller_reaches_run_end_through_the_same_choices() {
    let mut play = Play::default();
    play.admit();
    for action in [Action::InstallPalisade, Action::TrashBran] {
        let revision = play.observation().unwrap().decision.unwrap().revision;
        play.choose(revision, action).unwrap();
    }
    play.respond().unwrap();
    let revision = play.observation().unwrap().decision.unwrap().revision;
    play.choose(revision, Action::RezPalisade).unwrap();
    assert!(play.observation().unwrap().run_result.is_some());
}

#[test]
fn restarting_refuses_a_choice_from_the_previous_run() {
    let mut play = Play::default();
    play.admit();
    let stale = play.observation().unwrap().decision.unwrap().revision;
    play.choose(stale, Action::DeclineInstall).unwrap();
    play.restart().unwrap();
    let before = play.observation();
    assert!(play.choose(stale, Action::InstallPalisade).is_err());
    assert_eq!(play.observation(), before);
}
