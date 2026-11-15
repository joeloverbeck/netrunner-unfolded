use netrunner_unfolded::{
    game::{Action, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone},
    play::{Play, SaveOptions},
};

fn ready(play: &mut Play) {
    for _ in 0..12 {
        if play
            .observation()
            .unwrap()
            .decision
            .as_ref()
            .is_some_and(|d| d.choices.contains(&Action::GainCredit))
        {
            return;
        }
        play.respond().unwrap();
    }
    panic!("action window not reached");
}

fn choose(game: &mut Match, action: Action) -> u64 {
    let d = [Player::Corp, Player::Runner]
        .into_iter()
        .find_map(|p| game.observe(p).decision)
        .unwrap();
    assert!(
        d.choices.contains(&action),
        "{action:?} unavailable at {}: {:?}",
        game.observe(d.actor).phase,
        d.choices
    );
    game.choose(d.actor, d.revision, action).unwrap();
    d.revision + 1
}

fn until(game: &mut Match, phase: &str) {
    for _ in 0..40 {
        if game.observe(Player::Runner).phase.starts_with(phase) {
            return;
        }
        choose(game, Action::PassPriority);
    }
    panic!("did not reach {phase}");
}

#[test]
fn bran_install_restores_pending_cards_but_meaningful_opponent_response_seals_it() {
    let mut game = Match::prepared_bran_run(3);
    let before = game.observe(Player::Corp);
    let ice = before.hand[0].instance.unwrap();
    let bran = before.servers[0].ice[0].instance.unwrap();
    let revision = choose(&mut game, Action::InstallInward(ice));
    game.undo(Player::Corp, revision).unwrap();
    let mut restored = game.observe(Player::Corp);
    restored.decision.as_mut().unwrap().revision = before.decision.as_ref().unwrap().revision;
    assert_eq!(restored, before);
    choose(&mut game, Action::InstallInward(ice));
    choose(&mut game, Action::TrashInstalled(bran));
    choose(&mut game, Action::FinishInstall);
    assert!(game.undo_reason(Player::Corp).is_none());
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let revision = choose(&mut game, Action::Continue);
    let after = game.observe(Player::Corp);
    assert!(
        game.undo(Player::Corp, revision)
            .unwrap_err()
            .contains("opponent")
    );
    assert_eq!(game.observe(Player::Corp), after);
}

#[test]
fn manegarm_payment_restores_clicks_and_pending_effect_but_access_and_chance_seal_it() {
    let mut position = EngineeringPosition::economy(42);
    position.active = Player::Runner;
    position.runner_credits = 4;
    position.shuffle_decks = false;
    position.cards = vec![
        EngineeringCard {
            kind: CardKind::ManegarmSkunkworks,
            zone: Zone::Root(Server::Hq),
            rezzed: true,
            installed_turn: Some(1),
            advancement: 0,
        },
        EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
        EngineeringCard::new(CardKind::OffworldOffice, Zone::Hq),
    ];
    let mut game = Match::engineering_turns(position).unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Action::RunServer(Server::Hq));
    for _ in 0..4 {
        choose(&mut game, Action::PassPriority);
    }
    choose(&mut game, Action::Continue);
    until(&mut game, "Manegarm");
    let before = game.observe(Player::Runner);
    let mut uncorrected = Match::resume(game.checkpoint()).unwrap();
    let revision = choose(&mut game, Action::SpendManegarmClicks);
    assert_eq!(game.observe(Player::Runner).clicks[1], 1);
    game = Match::resume(game.checkpoint()).unwrap();
    game.undo(Player::Runner, revision).unwrap();
    let mut restored = game.observe(Player::Runner);
    restored.decision.as_mut().unwrap().revision = before.decision.as_ref().unwrap().revision;
    assert_eq!(restored, before);
    choose(&mut game, Action::SpendManegarmClicks);
    let root = choose(&mut game, Action::AccessRoot(0));
    assert!(
        game.undo(Player::Runner, root)
            .unwrap_err()
            .contains("new information")
    );
    choose(&mut game, Action::FinishAccess);
    let chance = choose(&mut game, Action::AccessHq);
    let observed = [game.observe(Player::Corp), game.observe(Player::Runner)];
    game = Match::resume(game.checkpoint()).unwrap();
    assert!(game.undo(Player::Runner, chance).is_err());
    assert_eq!(
        [game.observe(Player::Corp), game.observe(Player::Runner)],
        observed
    );
    for action in [
        Action::SpendManegarmClicks,
        Action::AccessRoot(0),
        Action::FinishAccess,
        Action::AccessHq,
    ] {
        choose(&mut uncorrected, action);
    }
    // Correction does not perturb the later committed random HQ selection.
    for viewer in [Player::Corp, Player::Runner] {
        let expected = uncorrected.observe(viewer);
        let mut actual = game.observe(viewer);
        if let (Some(actual), Some(expected)) = (&mut actual.decision, &expected.decision) {
            actual.revision = expected.revision;
        }
        assert_eq!(actual, expected);
    }
}

#[test]
fn random_net_damage_seals_the_last_eligible_choice_even_when_resolution_was_forced() {
    let mut position = EngineeringPosition::economy(42);
    position.active = Player::Runner;
    position.cards = vec![
        EngineeringCard {
            kind: CardKind::Tithe,
            zone: Zone::Protecting(Server::Hq),
            rezzed: true,
            installed_turn: Some(1),
            advancement: 0,
        },
        EngineeringCard::new(CardKind::SureGamble, Zone::Grip),
        EngineeringCard::new(CardKind::Jailbreak, Zone::Grip),
    ];
    let mut game = Match::engineering_turns(position).unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    choose(&mut game, Action::PassPriority);
    let revision = choose(&mut game, Action::PassPriority);
    assert_eq!(game.observe(Player::Runner).heap.len(), 1);
    let before = game.observe(Player::Runner);
    assert!(
        game.undo(Player::Runner, revision)
            .unwrap_err()
            .contains("random outcome")
    );
    assert_eq!(game.observe(Player::Runner), before);
}

#[test]
fn exhausted_revision_headroom_refuses_undo_without_consuming_the_saved_correction() {
    let mut game = Match::engineering_turns(EngineeringPosition::economy(42)).unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Action::GainCredit);
    let mut payload = serde_json::to_value(game.checkpoint()).unwrap();
    payload["state"]["revision"] = serde_json::json!(u64::MAX - 100);
    let mut game = Match::resume(serde_json::from_value(payload).unwrap()).unwrap();
    let before = serde_json::to_value(game.checkpoint()).unwrap();
    assert!(game.undo(Player::Corp, u64::MAX - 100).is_err());
    assert_eq!(serde_json::to_value(game.checkpoint()).unwrap(), before);
}

#[test]
fn expanded_bran_installation_restores_suspended_effect_and_rez_seals_correction() {
    let mut game = Match::engineering_turns(EngineeringPosition::hq_access(42)).unwrap();
    until(&mut game, "Action phase");
    let event = game
        .observe(Player::Runner)
        .hand
        .iter()
        .find(|c| c.printing == Some("30029"))
        .unwrap()
        .instance
        .unwrap();
    choose(&mut game, Action::PlayCard(event));
    choose(&mut game, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let corp = game.observe(Player::Corp);
    let palisade = corp
        .hand
        .iter()
        .find(|c| c.printing == Some("30072"))
        .unwrap()
        .instance
        .unwrap();
    let bran = corp.servers[0]
        .ice
        .iter()
        .find(|c| c.printing == Some("30039"))
        .unwrap()
        .instance
        .unwrap();
    let rev = choose(&mut game, Action::InstallInward(palisade));
    game = Match::resume(game.checkpoint()).unwrap();
    game.undo(Player::Corp, rev).unwrap();
    let mut restored = game.observe(Player::Corp);
    restored.decision.as_mut().unwrap().revision = corp.decision.as_ref().unwrap().revision;
    assert_eq!(restored, corp);
    choose(&mut game, Action::InstallInward(palisade));
    choose(&mut game, Action::TrashInstalled(bran));
    choose(&mut game, Action::FinishInstall);
    until(&mut game, "Movement");
    let response = choose(&mut game, Action::Continue);
    assert!(
        game.undo(Player::Corp, response)
            .unwrap_err()
            .contains("opponent")
    );
    until(&mut game, "Approach ICE");
    choose(&mut game, Action::PassPriority);
    let reveal = choose(&mut game, Action::Rez(palisade));
    assert!(
        game.undo(Player::Corp, reveal)
            .unwrap_err()
            .contains("new information")
    );
}

#[test]
fn economy_undo_restores_complete_choice_after_restart_and_rejects_stale_results() {
    let dir = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: dir.path().join("undo.json"),
        recover_from: None,
    };
    let mut position = EngineeringPosition::economy(42);
    position.active = Player::Runner;
    let mut play = Play::engineering(position, Some(options.clone()), Player::Runner);
    play.admit();
    ready(&mut play);
    let before = play.observation().unwrap();
    let revision = play.revision().unwrap();
    play.choose(revision, Action::GainCredit).unwrap();
    assert_eq!(play.observation().unwrap().runner_credits, 6);
    assert_eq!(play.observation().unwrap().clicks, [0, 3]);
    let stale = play.revision().unwrap();
    drop(play);
    let mut play = Play::load_turns(options, Player::Runner);
    play.admit();
    play.undo(stale).unwrap();
    let mut restored = play.observation().unwrap();
    assert!(restored.decision.as_ref().unwrap().revision > stale);
    restored.decision.as_mut().unwrap().revision = revision;
    assert_eq!(restored, before);
    assert!(play.choose(stale, Action::GainCredit).is_err());
    assert!(
        play.choose_for(Player::Corp, stale, Action::PassPriority)
            .is_err()
    );
    assert!(play.undo(stale).is_err());
}

#[test]
fn each_roles_private_draw_seals_undo_across_restart_without_disclosing_the_card() {
    for actor in [Player::Runner, Player::Corp] {
        let dir = tempfile::tempdir().unwrap();
        let options = SaveOptions {
            path: dir.path().join("draw.json"),
            recover_from: None,
        };
        let mut position = EngineeringPosition::economy(42);
        position.active = actor;
        let mut play = Play::engineering(position, Some(options.clone()), actor);
        play.admit();
        // Both players' legal decisions use the production controller.
        for _ in 0..12 {
            if play
                .observation()
                .unwrap()
                .decision
                .as_ref()
                .is_some_and(|d| d.choices.contains(&Action::Draw))
            {
                break;
            }
            if let Some(d) = play.observation().unwrap().decision {
                play.choose(d.revision, Action::PassPriority).unwrap();
            } else {
                play.respond().unwrap();
            }
        }
        let hidden_before = play.observe(actor.other()).unwrap().history;
        play.choose(play.revision().unwrap(), Action::Draw).unwrap();
        let after = play.observation().unwrap();
        let opponent = play.observe(actor.other()).unwrap();
        assert!(opponent.history.len() > hidden_before.len());
        assert!(
            opponent
                .history
                .iter()
                .all(|line| !line.contains(after.hand.last().unwrap().title.as_ref().unwrap()))
        );
        drop(play);
        let mut resumed = Play::load_turns(options, actor);
        resumed.admit();
        assert!(
            resumed
                .undo(resumed.revision().unwrap())
                .unwrap_err()
                .contains("new information")
        );
        assert_eq!(resumed.observation().unwrap(), after);
    }
}
