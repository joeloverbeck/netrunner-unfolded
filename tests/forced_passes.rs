use netrunner_unfolded::{
    game::{Action, CardKind, EngineeringCard, EngineeringPosition, Player, Server, Zone},
    play::{Admission, Play, SaveOptions},
};

fn runner_position() -> EngineeringPosition {
    let mut position = EngineeringPosition::economy(42);
    position.active = Player::Runner;
    position
}

fn act(play: &mut Play, action: Action) {
    let revision = play.observation().unwrap().decision.unwrap().revision;
    play.choose(revision, action).unwrap();
}

#[test]
fn human_pass_only_windows_advance_at_admission_and_after_opponent_and_action() {
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("turns.json"),
        recover_from: None,
    };
    let mut play = Play::engineering(runner_position(), Some(options.clone()), Player::Runner);
    play.admit();
    assert!(play.observation().unwrap().decision.is_none());
    play.respond().unwrap();
    assert!(play.observation().unwrap().decision.is_none());
    play.respond().unwrap();
    let ready = play.observation().unwrap();
    assert_eq!(ready.runner_credits, 5);
    assert_eq!(ready.clicks, [0, 4]);
    assert!(
        ready
            .decision
            .unwrap()
            .choices
            .contains(&Action::GainCredit)
    );
    act(&mut play, Action::GainCredit);
    let after = play.observation().unwrap();
    assert!(after.decision.is_none());
    assert_eq!(after.runner_credits, 6);
    assert_eq!(after.clicks, [0, 3]);
    drop(play);
    let mut reopened = Play::load_turns(options, Player::Runner);
    reopened.admit();
    assert_eq!(reopened.observation().unwrap(), after);
}

#[test]
fn scoring_opportunities_and_consequential_single_choices_remain_explicit() {
    let mut position = EngineeringPosition::economy(42);
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 4,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut play = Play::engineering(position.clone(), None, Player::Corp);
    play.admit();
    let choices = play.observation().unwrap().decision.unwrap().choices;
    assert!(choices.contains(&Action::PassPriority));
    assert!(
        choices
            .iter()
            .any(|action| matches!(action, Action::Score(_)))
    );

    position.active = Player::Runner;
    position.accessed_office = true;
    let mut access = Play::engineering(position, None, Player::Runner);
    access.admit();
    assert_eq!(
        access.observation().unwrap().decision.unwrap().choices,
        vec![Action::StealAgenda]
    );

    let mut install = Play::engineering(runner_position(), None, Player::Runner);
    install.admit();
    install.respond().unwrap();
    install.respond().unwrap();
    let cleaver = install
        .observation()
        .unwrap()
        .hand
        .into_iter()
        .find(|card| card.printing == Some("30006"))
        .unwrap()
        .instance
        .unwrap();
    act(&mut install, Action::InstallCard(cleaver));
    assert_eq!(
        install.observation().unwrap().decision.unwrap().choices,
        vec![Action::FinishInstall]
    );
    assert_eq!(install.observation().unwrap().runner_credits, 5);
}

#[test]
fn automatic_pass_save_failure_keeps_the_committed_state_and_stops_admission() {
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("turns.json"),
        recover_from: None,
    };
    // Corp was the human before the restart, so the initial Runner window is
    // still a pending opponent choice in this record.
    let mut original = Play::engineering(runner_position(), Some(options.clone()), Player::Corp);
    original.admit();
    let pending = original.observe(Player::Runner).unwrap();
    let bytes = std::fs::read(&options.path).unwrap();
    drop(original);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.path().join("turns.json.lock"))
        .unwrap();
    lock.lock().unwrap();
    let mut reopened = Play::load_turns(options.clone(), Player::Runner);
    reopened.admit();
    assert!(matches!(reopened.admission(), Admission::Refused(reason) if reason.contains("busy")));
    assert_eq!(reopened.observation().unwrap(), pending);
    assert_eq!(std::fs::read(&options.path).unwrap(), bytes);
    drop(lock);
    assert!(reopened.respond().is_err());
}
