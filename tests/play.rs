use netrunner_unfolded::{game::Action, play::Play};

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
