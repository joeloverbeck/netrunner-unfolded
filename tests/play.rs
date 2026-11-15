use netrunner_unfolded::{
    game::{Action, RunResult},
    play::Play,
};
fn choose(play: &mut Play, action: Action) {
    play.choose(play.revision().unwrap(), action).unwrap();
}
fn install(play: &mut Play) {
    let action = play
        .observation()
        .unwrap()
        .decision
        .unwrap()
        .choices
        .into_iter()
        .find(|a| matches!(a, Action::InstallInward(_)))
        .unwrap();
    choose(play, action);
}
fn trash_and_finish(play: &mut Play) {
    let action = play
        .observation()
        .unwrap()
        .decision
        .unwrap()
        .choices
        .into_iter()
        .find(|a| matches!(a, Action::TrashInstalled(_)))
        .unwrap();
    choose(play, action);
    choose(play, Action::FinishInstall);
}
fn rez(play: &mut Play) {
    for _ in 0..12 {
        if let Some(action) = play
            .observation()
            .unwrap()
            .decision
            .and_then(|d| d.choices.into_iter().find(|a| matches!(a, Action::Rez(_))))
        {
            choose(play, action);
            return;
        }
        play.respond().unwrap();
    }
    panic!("rez opportunity not reached");
}
#[test]
fn suspended_installation_resumes_through_the_native_controller() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    assert!(!path.exists());
    play.admit();
    install(&mut play);
    let pending = play.observation().unwrap();
    drop(play);
    let mut resumed = Play::saved(&path);
    assert!(resumed.observation().is_none());
    resumed.admit();
    assert_eq!(resumed.observation().unwrap(), pending);
    trash_and_finish(&mut resumed);
    rez(&mut resumed);
    assert_eq!(resumed.observation().unwrap().corp_credits, 0);
}
#[test]
fn failed_opponent_save_keeps_the_pending_decision_and_requires_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    play.admit();
    install(&mut play);
    trash_and_finish(&mut play);
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
        "failed operation must not silently retry"
    );
    let mut resumed = Play::saved(&path);
    resumed.admit();
    rez(&mut resumed);
    assert_eq!(resumed.observation().unwrap().corp_credits, 0);
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
    install(&mut first);
    let committed = std::fs::read(&path).unwrap();
    assert!(
        stale
            .choose(stale.revision().unwrap(), Action::DeclineInstall)
            .is_err()
    );
    assert_eq!(stale.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), committed);
    let previous = unfolded_record_replacement::previous_path(&path);
    std::fs::write(&previous, b"damaged previous").unwrap();
    let before = first.observation();
    assert!(
        first
            .choose(first.revision().unwrap(), Action::FinishInstall)
            .is_err()
    );
    assert_eq!(first.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), committed);
    assert_eq!(std::fs::read(&previous).unwrap(), b"damaged previous");
}
#[test]
fn run_end_cannot_reset_or_reshuffle_the_ordinary_match_or_replace_its_save() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("match.json");
    let mut play = Play::saved(&path);
    play.admit();
    let stale = play.revision().unwrap();
    choose(&mut play, Action::DeclineInstall);
    let bytes = std::fs::read(&path).unwrap();
    let before = play.observation();
    assert!(play.restart().unwrap_err().contains("cannot be reset"));
    assert!(play.choose(stale, Action::DeclineInstall).is_err());
    assert_eq!(play.observation(), before);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(play);
    let mut reopened = Play::saved(&path);
    reopened.admit();
    assert_eq!(reopened.observation(), before);
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
    assert!(play.choose(0, Action::DeclineInstall).is_err());
    play.admit();
    let before = play.observation().unwrap();
    play.refuse("required texture disappeared".into());
    assert!(
        play.choose(play.revision().unwrap(), Action::DeclineInstall)
            .is_err()
    );
    assert!(play.restart().is_err());
    assert_eq!(play.observation().unwrap(), before);
}
#[test]
fn the_native_controller_reaches_run_end_through_the_same_choices() {
    let mut play = Play::default();
    play.admit();
    install(&mut play);
    trash_and_finish(&mut play);
    rez(&mut play);
    for _ in 0..8 {
        if play.observation().unwrap().run_result.is_some() {
            break;
        }
        play.respond().unwrap();
    }
    assert_eq!(
        play.observation().unwrap().run_result,
        Some(RunResult::EndedByIce)
    );
}
