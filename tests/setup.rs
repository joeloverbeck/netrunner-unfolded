use netrunner_unfolded::game::{Action, Match, Player};

fn choose(game: &mut Match, player: Player, action: Action) {
    let decision = game.observe(player).decision.expect("acting player");
    game.choose(player, decision.revision, action).unwrap();
}

#[test]
fn mulligan_is_once_per_player_and_cannot_be_undone_or_rerolled_on_resume() {
    let mut game = Match::introductory(42);
    let before_runner = game.observe(Player::Runner).hand;
    let before_corp = game.observe(Player::Corp).hand;
    choose(&mut game, Player::Corp, Action::Mulligan);
    assert_ne!(game.observe(Player::Corp).hand, before_corp);
    assert_eq!(game.observe(Player::Runner).hand, before_runner);
    assert!(
        game.undo_reason(Player::Corp)
            .unwrap()
            .contains("information")
    );
    let mut resumed = Match::resume(game.checkpoint()).unwrap();
    choose(&mut game, Player::Runner, Action::Mulligan);
    choose(&mut resumed, Player::Runner, Action::Mulligan);
    assert_eq!(
        game.observe(Player::Runner),
        resumed.observe(Player::Runner)
    );
    assert!(game.observe(Player::Runner).decision.is_none());
    assert_eq!(game.observe(Player::Runner).hand.len(), 5);
    assert!(
        game.undo_reason(Player::Runner)
            .unwrap()
            .contains("information")
    );
    assert!(
        game.observe(Player::Corp)
            .history
            .iter()
            .all(|entry| !entry.contains("Sure Gamble"))
    );
}

#[test]
fn native_controller_reopens_the_chosen_seat_and_pending_mulligan() {
    use netrunner_unfolded::play::{Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("game.json"),
        recover_from: None,
    };
    let mut play = Play::introductory_seeded(42, Some(options.clone()), Player::Runner);
    assert!(!options.path.exists());
    play.admit();
    play.respond().unwrap();
    let before = play.observation().unwrap();
    assert_eq!(
        before.decision.as_ref().unwrap().choices,
        [Action::KeepHand, Action::Mulligan]
    );
    drop(play);
    let mut reopened = Play::introductory_seeded(999, Some(options), Player::Corp);
    reopened.admit();
    assert_eq!(reopened.human(), Player::Runner);
    assert_eq!(reopened.observation().unwrap(), before);
    reopened
        .choose(before.decision.unwrap().revision, Action::Mulligan)
        .unwrap();
    assert!(reopened.undo_reason().unwrap().contains("information"));
}

#[test]
fn seeded_ordinary_games_finish_in_both_seats_and_survive_a_midgame_restart() {
    use netrunner_unfolded::{
        game::baseline_response,
        play::{Admission, Play, SaveOptions},
    };
    for human in [Player::Corp, Player::Runner] {
        for seed in [3, 42, 89, 117] {
            let directory = tempfile::tempdir().unwrap();
            let options = SaveOptions {
                path: directory.path().join("game.json"),
                recover_from: None,
            };
            let mut play = Play::introductory_seeded(seed, Some(options.clone()), human);
            play.admit();
            assert_eq!(play.admission(), &Admission::Ready);
            for step in 0..4000 {
                let view = play.observation().unwrap();
                if let Some(winner) = view.match_winner {
                    assert!(
                        view.agenda_points[winner.index()] >= 6
                            || matches!(
                                view.terminal_reason,
                                Some("Corp could not draw from R&D" | "Runner flatlined")
                            )
                    );
                    eprintln!(
                        "seed={seed} human={human:?} steps={step} winner={winner:?} reason={:?}",
                        view.terminal_reason
                    );
                    break;
                }
                if let Some(decision) = &view.decision {
                    let action = baseline_response(&view)
                        .expect("baseline must handle each lawful decision");
                    play.choose(decision.revision, action).unwrap_or_else(|error| panic!("seed={seed} human={human:?} step={step} phase={} action={action:?}: {error}", view.phase));
                } else {
                    assert!(
                        play.opponent_pending(),
                        "seed={seed} human={human:?} step={step} stuck in {}",
                        view.phase
                    );
                    play.respond().unwrap_or_else(|error| {
                        panic!(
                            "seed={seed} human={human:?} step={step} phase={}: {error}",
                            view.phase
                        )
                    });
                }
                if step == 50 {
                    let before = play.observation();
                    drop(play);
                    play = Play::introductory_seeded(9999, Some(options.clone()), human.other());
                    play.admit();
                    assert_eq!(play.observation(), before);
                    assert_eq!(play.human(), human);
                }
            }
            assert!(
                play.observation().unwrap().match_winner.is_some(),
                "seed={seed} human={human:?}: decision bound exhausted"
            );
        }
    }
}

#[test]
fn role_selection_precedes_match_creation_and_refuses_later_seat_changes() {
    use netrunner_unfolded::play::{Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("game.json"),
        recover_from: None,
    };
    let mut play =
        Play::introductory_seeded(5, Some(options.clone()), Player::Corp).with_role_selection();
    play.admit();
    assert!(play.needs_seat());
    assert!(play.observation().is_none());
    assert!(!options.path.exists());
    play.select_seat(Player::Runner).unwrap();
    assert_eq!(play.human(), Player::Runner);
    assert!(!play.needs_seat());
    assert!(options.path.exists());
    let before = play.observation();
    assert!(play.select_seat(Player::Corp).is_err());
    assert_eq!(play.observation(), before);
    drop(play);
    let mut reopened =
        Play::introductory_seeded(6, Some(options), Player::Corp).with_role_selection();
    reopened.admit();
    assert!(!reopened.needs_seat());
    assert_eq!(reopened.human(), Player::Runner);
    assert_eq!(reopened.observation(), before);
}

#[test]
fn every_card_in_the_official_decks_is_drawable_with_exact_multiplicity() {
    use netrunner_unfolded::game::CardId;
    use std::collections::BTreeMap;
    let mut game = Match::introductory(71);
    let mut seen: Vec<(CardId, String)> = vec![];
    for _ in 0..1500 {
        for player in [Player::Corp, Player::Runner] {
            let view = game.observe(player);
            for card in &view.hand {
                let id = card.instance.unwrap();
                if !seen.iter().any(|(known, _)| *known == id) {
                    seen.push((id, card.title.clone().unwrap()));
                }
            }
        }
        if game.observe(Player::Corp).match_winner.is_some() {
            break;
        }
        let player = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        let view = game.observe(player);
        let choices = &view.decision.as_ref().unwrap().choices;
        let action = [Action::KeepHand, Action::Draw, Action::ConfirmDiscard, Action::PassPriority]
            .into_iter().find(|a| choices.contains(a))
            .or_else(|| choices.iter().find(|a| matches!(a, Action::ToggleDiscard(id) if view.hand.iter().any(|c| c.instance == Some(*id) && !c.selected))).copied())
            .unwrap();
        choose(&mut game, player, action);
    }
    let mut counts = BTreeMap::new();
    for (_, title) in seen {
        *counts.entry(title).or_insert(0) += 1;
    }
    let expected = [
        ("Tread Lightly", 2),
        ("Creative Commission", 2),
        ("VRcation", 2),
        ("Overclock", 2),
        ("Jailbreak", 3),
        ("Sure Gamble", 3),
        ("Docklands Pass", 1),
        ("Pennyshaver", 1),
        ("Cleaver", 2),
        ("Carmen", 2),
        ("Unity", 2),
        ("Mayfly", 2),
        ("Red Team", 1),
        ("Telework Contract", 2),
        ("Smartware Distributor", 2),
        ("Verbal Plasticity", 1),
        ("Offworld Office", 3),
        ("Send a Message", 2),
        ("Superconducting Hub", 2),
        ("Nico Campaign", 2),
        ("Urtica Cipher", 2),
        ("Regolith Mining License", 2),
        ("Brân 1.0", 2),
        ("Diviner", 2),
        ("Karunā", 2),
        ("Palisade", 3),
        ("Tithe", 2),
        ("Whitespace", 2),
        ("Seamless Launch", 2),
        ("Government Subsidy", 2),
        ("Hedge Fund", 3),
        ("Manegarm Skunkworks", 1),
    ]
    .into_iter()
    .map(|(title, count)| (title.to_owned(), count))
    .collect();
    assert_eq!(counts, expected);
    assert_eq!(
        game.observe(Player::Corp).terminal_reason,
        Some("Corp could not draw from R&D")
    );
}

#[test]
fn unsupported_decks_and_engineering_saves_cannot_enter_ordinary_play() {
    use netrunner_unfolded::{
        game::EngineeringPosition,
        play::{Admission, Play, SaveOptions},
    };
    let game = Match::introductory(1);
    let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
    checkpoint["state"]["cards"].as_array_mut().unwrap().pop();
    assert!(Match::resume(serde_json::from_value(checkpoint).unwrap()).is_err());
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("game.json"),
        recover_from: None,
    };
    let mut fixture = Play::engineering(
        EngineeringPosition::economy(1),
        Some(options.clone()),
        Player::Corp,
    );
    fixture.admit();
    let before = std::fs::read(&options.path).unwrap();
    let mut ordinary = Play::introductory_seeded(2, Some(options.clone()), Player::Corp);
    ordinary.admit();
    assert!(matches!(ordinary.admission(), Admission::Refused(_)));
    assert!(ordinary.observation().is_none());
    assert_eq!(std::fs::read(&options.path).unwrap(), before);
}

#[test]
fn live_new_match_keeps_the_old_game_until_a_seat_commits_and_archives_it_durably() {
    use netrunner_unfolded::play::{OPPONENT_BUDGET, Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("live.json"),
        recover_from: None,
    };
    let mut play = Play::introductory_seeded(42, Some(options.clone()), Player::Runner);
    play.admit();
    let old_view = play.observation().unwrap();
    let old_revision = play.revision().unwrap();
    let old_bytes = std::fs::read(&options.path).unwrap();
    let old_work = play.prepare_opponent(OPPONENT_BUDGET).unwrap().unwrap();
    play.begin_new_match_seeded(89).unwrap();
    assert!(play.needs_seat());
    assert_eq!(std::fs::read(&options.path).unwrap(), old_bytes);
    assert!(
        play.choose_for(Player::Corp, old_revision, Action::KeepHand)
            .is_err()
    );
    assert!(play.commit_opponent(old_work.compute()).is_err());
    drop(play); // Quit before choosing a role must reopen the original match.
    let mut play = Play::introductory_seeded(1, Some(options.clone()), Player::Corp);
    play.admit();
    assert_eq!(play.observation().unwrap(), old_view);
    let stale = play
        .prepare_opponent(OPPONENT_BUDGET)
        .unwrap()
        .unwrap()
        .compute();
    play.begin_new_match_seeded(89).unwrap();
    play.select_seat(Player::Corp).unwrap();
    assert!(play.revision().unwrap() > old_revision);
    assert_eq!(
        play.observation().unwrap().hand,
        Match::introductory(89).observe(Player::Corp).hand
    );
    assert!(play.commit_opponent(stale).is_err());
    let archives: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("live.json.archived-")
        })
        .collect();
    assert_eq!(
        archives.len(),
        1,
        "new match retains a separate durable archive"
    );
    assert_eq!(play.archived_match(), Some(archives[0].as_path()));
    assert_eq!(std::fs::read(&archives[0]).unwrap(), old_bytes);
    let revision = play.revision().unwrap();
    play.choose(revision, Action::KeepHand).unwrap();
    assert_eq!(
        std::fs::read(&archives[0]).unwrap(),
        old_bytes,
        "later save rotation cannot overwrite the abandoned match"
    );
    let mut archived = Play::introductory_seeded(
        1,
        Some(SaveOptions {
            path: archives[0].clone(),
            recover_from: None,
        }),
        Player::Corp,
    );
    archived.admit();
    assert_eq!(archived.observation().unwrap(), old_view);
    archived.respond().unwrap(); // The archived record remains playable.
}

#[test]
fn live_new_match_refuses_an_externally_changed_save_before_archiving() {
    use netrunner_unfolded::play::{Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("live.json"),
        recover_from: None,
    };
    let mut first = Play::introductory_seeded(42, Some(options.clone()), Player::Runner);
    first.admit();
    first.begin_new_match_seeded(89).unwrap();
    let mut second = Play::introductory_seeded(1, Some(options.clone()), Player::Runner);
    second.admit();
    second.respond().unwrap();
    let newer = std::fs::read(&options.path).unwrap();
    assert!(
        first
            .select_seat(Player::Corp)
            .unwrap_err()
            .contains("changed")
    );
    assert_eq!(std::fs::read(&options.path).unwrap(), newer);
    assert!(first.archived_match().is_none());
    assert!(!std::fs::read_dir(directory.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".archived-")
    }));
}

#[test]
fn new_match_waits_for_a_seat_then_preserves_old_save_and_rejects_old_choices() {
    use netrunner_unfolded::{
        game::baseline_response,
        play::{Play, SaveOptions},
    };
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("game.json"),
        recover_from: None,
    };
    let mut play = Play::introductory_seeded(42, Some(options.clone()), Player::Corp);
    play.admit();
    for _ in 0..1000 {
        let view = play.observation().unwrap();
        if view.match_winner.is_some() {
            break;
        }
        if let Some(decision) = &view.decision {
            play.choose(decision.revision, baseline_response(&view).unwrap())
                .unwrap();
        } else {
            play.respond().unwrap();
        }
    }
    assert!(play.observation().unwrap().match_winner.is_some());
    let ended = std::fs::read(&options.path).unwrap();
    let old_revision = play.revision().unwrap();
    play.begin_new_match_seeded(89).unwrap();
    assert!(play.needs_seat());
    assert!(play.observation().is_none());
    assert_eq!(std::fs::read(&options.path).unwrap(), ended);
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.path().join("game.json.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(
        play.select_seat(Player::Runner)
            .unwrap_err()
            .contains("busy")
    );
    assert_eq!(std::fs::read(&options.path).unwrap(), ended);
    assert!(play.observation().unwrap().match_winner.is_some());
    drop(lock);
    drop(play);
    let mut play = Play::introductory_seeded(1, Some(options.clone()), Player::Corp);
    play.admit();
    play.begin_new_match_seeded(89).unwrap();
    play.select_seat(Player::Runner).unwrap();
    assert!(play.revision().unwrap() > old_revision);
    assert_eq!(
        play.observe(Player::Runner).unwrap().hand,
        Match::introductory(89).observe(Player::Runner).hand
    );
    assert!(
        play.choose_for(Player::Corp, old_revision, Action::KeepHand)
            .is_err()
    );
    assert_eq!(
        std::fs::read(options.path.with_extension("json.previous")).unwrap(),
        ended
    );
    drop(play);
    let mut resumed =
        Play::introductory_seeded(1, Some(options), Player::Corp).with_role_selection();
    resumed.admit();
    assert_eq!(resumed.human(), Player::Runner);
    assert_eq!(resumed.observation().unwrap().turn_number, 0);
}

// CR 26.03 1.6.4–1.6.7: five credits and cards each, Corp decides
// first, then Runner, then the Corp's first turn (including mandatory draw).
#[test]
fn introductory_setup_deals_private_hands_then_enters_the_first_corp_turn() {
    let mut game = Match::introductory(42);
    let corp = game.observe(Player::Corp);
    let runner = game.observe(Player::Runner);
    assert_eq!((corp.hq_count, corp.rd_count), (5, 29));
    assert_eq!((runner.grip_count, runner.stack_count), (5, 25));
    assert_eq!((corp.corp_credits, corp.runner_credits), (5, 5));
    assert_eq!(corp.clicks, [0, 0]);
    assert_eq!(corp.turn_number, 0);
    assert_eq!(corp.hand.len(), 5);
    assert_eq!(runner.hand.len(), 5);
    assert_eq!(
        corp.decision.unwrap().choices,
        [Action::KeepHand, Action::Mulligan]
    );
    assert!(runner.decision.is_none());
    choose(&mut game, Player::Corp, Action::KeepHand);
    assert!(game.observe(Player::Corp).decision.is_none());
    choose(&mut game, Player::Runner, Action::KeepHand);
    assert_eq!(game.observe(Player::Corp).turn_number, 1);
    assert_eq!(game.observe(Player::Corp).clicks, [3, 0]);
    choose(&mut game, Player::Corp, Action::PassPriority);
    choose(&mut game, Player::Runner, Action::PassPriority);
    let corp = game.observe(Player::Corp);
    assert_eq!((corp.hq_count, corp.rd_count), (6, 28));
    assert_eq!(corp.corp_identity.printing, Some("30077"));
    assert_eq!(corp.runner_identity.printing, Some("30076"));
}
