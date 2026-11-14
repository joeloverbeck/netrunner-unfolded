use netrunner_unfolded::game::{
    Action, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Zone,
};
#[test]
fn overclock_pays_one_pool_credit_and_suspends_at_server_choice() {
    let mut position = EngineeringPosition::economy(42);
    position.active = Player::Runner;
    position.runner_credits = 8;
    let overclock: CardKind = serde_json::from_str("\"Overclock\"").unwrap();
    position
        .cards
        .push(EngineeringCard::new(overclock, Zone::Grip));
    let mut game = Match::engineering_turns(position).unwrap();
    for _ in 0..4 {
        let actor = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        let revision = game.observe(actor).decision.unwrap().revision;
        game.choose(actor, revision, Action::PassPriority).unwrap();
    }
    let view = game.observe(Player::Runner);
    let card = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30029"))
        .unwrap()
        .instance
        .unwrap();
    game.choose(
        Player::Runner,
        view.decision.unwrap().revision,
        Action::PlayCard(card),
    )
    .unwrap();
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 7);
    assert_eq!(view.clicks[1], 3);
    assert_eq!(view.phase, "Overclock · choose server");
}

fn choose(game: &mut Match, player: Player, action: Action) {
    let view = game.observe(player);
    let decision = view.decision.expect("actor has decision");
    assert!(
        decision.choices.contains(&action),
        "{action:?} not offered in {}: {:?}",
        view.phase,
        decision.choices
    );
    game.choose(player, decision.revision, action).unwrap();
    let restored = Match::resume(game.checkpoint()).unwrap();
    for viewer in [Player::Corp, Player::Runner] {
        assert_eq!(game.observe(viewer), restored.observe(viewer));
    }
}
fn passes(game: &mut Match) {
    for _ in 0..2 {
        let player = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        choose(game, player, Action::PassPriority);
    }
}
fn installed(kind: CardKind, zone: Zone) -> EngineeringCard {
    EngineeringCard {
        kind,
        zone,
        advancement: 0,
        installed_turn: Some(1),
        rezzed: zone != Zone::Rig,
    }
}
fn worked_position() -> EngineeringPosition {
    use netrunner_unfolded::game::Server;
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.runner_credits = 8;
    p.corp_credits = 3;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::Overclock, Zone::Grip),
        EngineeringCard::new(CardKind::Palisade, Zone::Hq),
        installed(CardKind::Whitespace, Zone::Protecting(Server::Hq)),
        installed(CardKind::Bran, Zone::Protecting(Server::Hq)),
        installed(CardKind::Cleaver, Zone::Rig),
        installed(CardKind::Unity, Zone::Rig),
    ];
    p
}
fn card(game: &Match, player: Player, title: &str) -> netrunner_unfolded::game::CardId {
    let v = game.observe(player);
    v.hand
        .iter()
        .chain(&v.rig)
        .chain(&v.archives)
        .chain(v.servers.iter().flat_map(|s| &s.ice))
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}
fn pass_until(game: &mut Match, phase: &str) {
    for _ in 0..16 {
        if game.observe(Player::Runner).phase.starts_with(phase) {
            return;
        }
        let actor = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        choose(game, actor, Action::PassPriority);
    }
    panic!("phase not reached: {phase}");
}
fn start(game: &mut Match) {
    passes(game);
    passes(game);
    let id = card(game, Player::Runner, "Overclock");
    choose(game, Player::Runner, Action::PlayCard(id));
    choose(
        game,
        Player::Runner,
        Action::RunServer(netrunner_unfolded::game::Server::Hq),
    );
    pass_until(game, "Encounter");
}
#[test]
fn parent_worked_arithmetic_preserves_inward_position_and_distinguishes_hosted_credits() {
    let mut game = Match::engineering_turns(worked_position()).unwrap();
    start(&mut game);
    passes(&mut game);
    let palisade = card(&game, Player::Corp, "Palisade");
    let bran = card(&game, Player::Corp, "Brân 1.0");
    choose(&mut game, Player::Corp, Action::InstallInward(palisade));
    choose(&mut game, Player::Corp, Action::TrashInstalled(bran));
    choose(&mut game, Player::Corp, Action::FinishInstall);
    pass_until(&mut game, "Movement");
    let runner = game.observe(Player::Runner);
    assert_eq!(runner.servers[0].ice.len(), 2);
    assert!(runner.servers[0].ice[1].title.is_none());
    choose(&mut game, Player::Runner, Action::Continue);
    pass_until(&mut game, "Approach ICE");
    choose(&mut game, Player::Runner, Action::PassPriority);
    choose(&mut game, Player::Corp, Action::Rez(palisade));
    passes(&mut game);
    let cleaver = card(&game, Player::Runner, "Cleaver");
    choose(
        &mut game,
        Player::Runner,
        Action::Break {
            breaker: cleaver,
            subroutines: 1,
        },
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(1));
    pass_until(&mut game, "Movement");
    assert_eq!(game.observe(Player::Runner).run.unwrap().hosted_credits, 4);
    choose(&mut game, Player::Runner, Action::Continue);
    pass_until(&mut game, "Encounter");
    let unity = card(&game, Player::Runner, "Unity");
    choose(
        &mut game,
        Player::Runner,
        Action::Break {
            breaker: unity,
            subroutines: 2,
        },
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(1));
    pass_until(&mut game, "Movement");
    let runner = game.observe(Player::Runner);
    assert_eq!(runner.runner_credits, 4);
    assert_eq!(runner.corp_credits, 0);
    assert_eq!(runner.clicks[1], 3);
    assert_eq!(runner.run.unwrap().hosted_credits, 3);
    choose(&mut game, Player::Runner, Action::Continue);
    // #14 now completes an empty HQ breach rather than retaining #13's
    // unavailable continuation boundary. The completed encounter arithmetic
    // above remains the independently derived #13 obligation.
    passes(&mut game);
    assert!(game.observe(Player::Runner).run.is_none());
    assert_eq!(
        game.observe(Player::Runner).run_result,
        Some(netrunner_unfolded::game::RunResult::SuccessfulAccess)
    );
}

#[test]
fn ordinary_run_costs_one_click_and_boosts_outside_encounters_expire() {
    let mut game = Match::engineering_turns(worked_position()).unwrap();
    passes(&mut game);
    let unity = card(&game, Player::Runner, "Unity");
    choose(&mut game, Player::Runner, Action::Boost(unity));
    assert_eq!(game.observe(Player::Runner).runner_credits, 7);
    passes(&mut game);
    choose(
        &mut game,
        Player::Runner,
        Action::RunServer(netrunner_unfolded::game::Server::Hq),
    );
    assert_eq!(game.observe(Player::Runner).clicks[1], 3);
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[1].2, 1);
}

#[test]
fn boosts_in_initiation_and_unrezzed_approach_can_spend_hosted_credits_but_do_not_carry() {
    let mut p = worked_position();
    for c in &mut p.cards {
        if c.kind == CardKind::Bran {
            c.rezzed = false;
        }
    }
    let mut game = Match::engineering_turns(p).unwrap();
    passes(&mut game);
    passes(&mut game);
    let event = card(&game, Player::Runner, "Overclock");
    choose(&mut game, Player::Runner, Action::PlayCard(event));
    choose(
        &mut game,
        Player::Runner,
        Action::RunServer(netrunner_unfolded::game::Server::Hq),
    );
    let unity = card(&game, Player::Runner, "Unity");
    for _ in 0..2 {
        choose(&mut game, Player::Runner, Action::Boost(unity));
        choose(&mut game, Player::Runner, Action::PayRunCredits(1));
        assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[1].2, 1);
        passes(&mut game);
    }
    assert_eq!(game.observe(Player::Runner).run.unwrap().hosted_credits, 3);
}

#[test]
fn suspended_run_admission_rejects_impossible_subroutines_and_payment_targets() {
    let mut game = Match::engineering_turns(worked_position()).unwrap();
    start(&mut game);
    let unity = card(&game, Player::Runner, "Unity");
    choose(&mut game, Player::Runner, Action::Boost(unity));
    for (pointer, value) in [
        ("/state/turn/run/broken", serde_json::json!(128)),
        ("/state/turn/run/stage/Payment/cost", serde_json::json!(0)),
        (
            "/state/turn/run/stage/Payment/effect",
            serde_json::json!({"Boost":999}),
        ),
        ("/state/turn/run/boosts", serde_json::json!([[999, 1]])),
        ("/state/turn/run/cursor", serde_json::json!(999)),
    ] {
        let mut json = serde_json::to_value(game.checkpoint()).unwrap();
        *json.pointer_mut(pointer).unwrap() = value;
        assert!(
            Match::resume(serde_json::from_value(json).unwrap()).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn bran_clicks_break_selected_subroutines_and_declining_install_preserves_a_surviving_run() {
    let mut game = Match::engineering_turns(worked_position()).unwrap();
    start(&mut game);
    choose(&mut game, Player::Runner, Action::BreakWithClick(2));
    choose(&mut game, Player::Runner, Action::BreakWithClick(3));
    let view = game.observe(Player::Runner);
    assert_eq!(view.clicks[1], 1);
    assert_eq!(view.runner_credits, 7);
    assert!(!view.run.unwrap().subroutines[0].broken);
    passes(&mut game);
    choose(&mut game, Player::Corp, Action::DeclineInstall);
    pass_until(&mut game, "Movement");
    assert_eq!(game.observe(Player::Runner).run_result, None);
    choose(&mut game, Player::Runner, Action::JackOut);
    let view = game.observe(Player::Runner);
    assert_eq!(
        view.run_result,
        Some(netrunner_unfolded::game::RunResult::JackedOut)
    );
    assert!(view.run.is_none());
    assert!(view.heap.iter().any(|c| c.printing == Some("30029")));
}

#[test]
fn retaining_bran_or_declining_install_with_unbroken_end_run_stops_the_run() {
    for install in [false, true] {
        let mut game = Match::engineering_turns(worked_position()).unwrap();
        start(&mut game);
        passes(&mut game);
        if install {
            let id = card(&game, Player::Corp, "Palisade");
            choose(&mut game, Player::Corp, Action::InstallInward(id));
            choose(&mut game, Player::Corp, Action::FinishInstall);
        } else {
            choose(&mut game, Player::Corp, Action::DeclineInstall);
        }
        let v = game.observe(Player::Runner);
        assert_eq!(
            v.run_result,
            Some(netrunner_unfolded::game::RunResult::EndedByIce)
        );
        assert_eq!(v.servers[0].ice.len(), if install { 3 } else { 2 });
        assert_eq!(v.corp_credits, 3);
        assert_eq!(v.runner_credits, 7);
    }
}

#[test]
fn subtype_strength_and_selected_subroutines_gate_breaking_with_mixed_payment() {
    let mut game = Match::engineering_turns(worked_position()).unwrap();
    start(&mut game);
    let cleaver = card(&game, Player::Runner, "Cleaver");
    let unity = card(&game, Player::Runner, "Unity");
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .iter()
            .any(|a| matches!(a, Action::Break { .. }))
    );
    choose(&mut game, Player::Runner, Action::Boost(unity));
    choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[1].2, 3);
    for _ in 0..3 {
        choose(&mut game, Player::Runner, Action::Boost(cleaver));
        choose(&mut game, Player::Runner, Action::PayRunCredits(1));
    }
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 3);
    assert_eq!(view.run.as_ref().unwrap().hosted_credits, 2);
    assert_eq!(view.run.as_ref().unwrap().breakers[0].2, 6);
    assert!(
        !view
            .decision
            .as_ref()
            .unwrap()
            .choices
            .contains(&Action::Break {
                breaker: unity,
                subroutines: 1
            })
    );
    choose(
        &mut game,
        Player::Runner,
        Action::Break {
            breaker: cleaver,
            subroutines: 6,
        },
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(1));
    let run = game.observe(Player::Runner).run.unwrap();
    assert!(!run.subroutines[0].broken);
    assert!(run.subroutines[1].broken && run.subroutines[2].broken);
    passes(&mut game);
    choose(&mut game, Player::Corp, Action::DeclineInstall);
    pass_until(&mut game, "Movement");
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[0].2, 3);
}

#[test]
fn whitespace_resolves_loss_then_threshold_without_between_subroutine_choices() {
    for (pool, survives, remaining) in [(8, false, 4), (11, true, 7), (2, false, 0)] {
        let mut p = worked_position();
        p.runner_credits = pool;
        p.cards.retain(|c| c.kind != CardKind::Bran);
        let mut game = Match::engineering_turns(p).unwrap();
        start(&mut game);
        passes(&mut game);
        let view = game.observe(Player::Runner);
        assert_eq!(view.runner_credits, remaining);
        assert_eq!(view.run_result.is_none(), survives);
        if survives {
            assert_eq!(view.run.unwrap().hosted_credits, 5);
            assert_eq!(view.phase, "ICE passed · paid window");
        } else {
            assert!(view.run.is_none());
            assert!(view.heap.iter().any(|c| c.printing == Some("30029")));
        }
    }
}

#[test]
fn archives_install_can_trash_multiple_same_server_ice_and_cannot_rez_without_credits() {
    let mut p = worked_position();
    p.corp_credits = 2;
    for c in &mut p.cards {
        if c.kind == CardKind::Palisade {
            c.zone = Zone::Archives;
        }
    }
    let mut game = Match::engineering_turns(p).unwrap();
    start(&mut game);
    passes(&mut game);
    let palisade = card(&game, Player::Corp, "Palisade");
    let bran = card(&game, Player::Corp, "Brân 1.0");
    let whitespace = card(&game, Player::Corp, "Whitespace");
    choose(&mut game, Player::Corp, Action::InstallInward(palisade));
    choose(&mut game, Player::Corp, Action::TrashInstalled(whitespace));
    choose(&mut game, Player::Corp, Action::TrashInstalled(bran));
    choose(&mut game, Player::Corp, Action::FinishInstall);
    pass_until(&mut game, "Movement");
    choose(&mut game, Player::Runner, Action::Continue);
    pass_until(&mut game, "Approach ICE");
    choose(&mut game, Player::Runner, Action::PassPriority);
    let view = game.observe(Player::Corp);
    assert_eq!(view.decision.unwrap().choices, vec![Action::PassPriority]);
    choose(&mut game, Player::Corp, Action::PassPriority);
    pass_until(&mut game, "Movement");
    assert_eq!(game.observe(Player::Runner).servers[0].ice.len(), 1);
    assert!(
        game.observe(Player::Runner).servers[0].ice[0]
            .title
            .is_none()
    );
}

#[test]
fn hidden_ice_and_hq_faces_do_not_enter_runner_run_inputs_or_policy() {
    let mut first = worked_position();
    for c in &mut first.cards {
        if c.kind == CardKind::Bran {
            c.rezzed = false;
        }
    }
    let mut second = first.clone();
    for c in &mut second.cards {
        if c.zone == Zone::Hq {
            c.kind = CardKind::Whitespace;
        }
        if c.kind == CardKind::Bran {
            c.kind = CardKind::Palisade;
        }
    }
    let mut a = Match::engineering_turns(first).unwrap();
    let mut b = Match::engineering_turns(second).unwrap();
    for game in [&mut a, &mut b] {
        passes(game);
        passes(game);
        let event = card(game, Player::Runner, "Overclock");
        choose(game, Player::Runner, Action::PlayCard(event));
        choose(
            game,
            Player::Runner,
            Action::RunServer(netrunner_unfolded::game::Server::Hq),
        );
        passes(game);
    }
    assert_ne!(a.observe(Player::Corp), b.observe(Player::Corp));
    assert_eq!(a.observe(Player::Runner), b.observe(Player::Runner));
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&a.observe(Player::Runner)),
        netrunner_unfolded::game::baseline_response(&b.observe(Player::Runner))
    );
}

#[test]
fn no_funds_means_no_break_or_boost_but_bran_clicks_remain_legal() {
    let mut p = worked_position();
    p.runner_credits = 0;
    let mut game = Match::engineering_turns(p).unwrap();
    passes(&mut game);
    passes(&mut game);
    choose(
        &mut game,
        Player::Runner,
        Action::RunServer(netrunner_unfolded::game::Server::Hq),
    );
    pass_until(&mut game, "Encounter");
    let view = game.observe(Player::Runner);
    let choices = &view.decision.as_ref().unwrap().choices;
    assert!(choices.contains(&Action::BreakWithClick(1)));
    assert!(
        !choices
            .iter()
            .any(|a| matches!(a, Action::Boost(_) | Action::Break { .. }))
    );
    let before = game.observe(Player::Runner);
    let unity = card(&game, Player::Runner, "Unity");
    assert!(
        game.choose(
            Player::Runner,
            before.decision.as_ref().unwrap().revision,
            Action::Boost(unity)
        )
        .is_err()
    );
    assert_eq!(game.observe(Player::Runner), before);
}

#[test]
fn a_paid_ability_can_reset_priority_on_the_other_players_turn() {
    let mut p = worked_position();
    p.active = Player::Corp;
    p.cards
        .push(EngineeringCard::new(CardKind::HedgeFund, Zone::Rd));
    let mut game = Match::engineering_turns(p).unwrap();
    choose(&mut game, Player::Corp, Action::PassPriority);
    let unity = card(&game, Player::Runner, "Unity");
    choose(&mut game, Player::Runner, Action::Boost(unity));
    choose(&mut game, Player::Runner, Action::PassPriority);
    choose(&mut game, Player::Corp, Action::PassPriority);
    assert_eq!(game.observe(Player::Corp).hq_count, 2);
    assert_eq!(game.observe(Player::Runner).runner_credits, 7);
}

#[test]
fn overclock_is_visible_with_five_hosted_credits_before_the_server_is_chosen() {
    let mut game = Match::engineering_turns(worked_position()).unwrap();
    passes(&mut game);
    passes(&mut game);
    let event = card(&game, Player::Runner, "Overclock");
    choose(&mut game, Player::Runner, Action::PlayCard(event));
    let view = serde_json::to_value(game.observe(Player::Corp)).unwrap();
    assert_eq!(view["play_area"][0]["printing"], "30029");
    assert_eq!(view["play_area"][0]["hosted_credits"], 5);
}

#[test]
fn prior_turn_engine_records_are_refused_without_replacing_them() {
    use netrunner_unfolded::play::{Admission, Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let options = SaveOptions {
        path: directory.path().join("older.json"),
        recover_from: None,
    };
    let mut play = Play::engineering(worked_position(), Some(options.clone()), Player::Runner);
    play.admit();
    drop(play);
    let mut envelope: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&options.path).unwrap()).unwrap();
    envelope["engine"] = serde_json::json!("ordinary-turns-1");
    let bytes = serde_json::to_vec(&envelope).unwrap();
    std::fs::write(&options.path, &bytes).unwrap();
    let mut newer = Play::load_turns(options.clone(), Player::Runner);
    newer.admit();
    assert!(
        matches!(newer.admission(),Admission::Refused(reason)if reason.contains("Unsupported save"))
    );
    assert_eq!(std::fs::read(&options.path).unwrap(), bytes);
}

#[test]
fn repeated_bran_replacement_retires_vacant_positions_when_the_runner_moves() {
    let mut p = worked_position();
    p.corp_credits = 36;
    p.cards.retain(|c| c.kind != CardKind::Whitespace);
    for c in &mut p.cards {
        if c.kind == CardKind::Palisade {
            c.kind = CardKind::Bran;
        }
    }
    let mut game = Match::engineering_turns(p).unwrap();
    start(&mut game);
    for _ in 0..6 {
        passes(&mut game);
        let view = game.observe(Player::Corp);
        let source = view.servers[0].ice[0].instance.unwrap();
        let replacement = view
            .hand
            .iter()
            .chain(&view.archives)
            .find(|c| c.printing == Some("30039"))
            .unwrap()
            .instance
            .unwrap();
        choose(&mut game, Player::Corp, Action::InstallInward(replacement));
        choose(&mut game, Player::Corp, Action::TrashInstalled(source));
        choose(&mut game, Player::Corp, Action::FinishInstall);
        pass_until(&mut game, "Movement");
        choose(&mut game, Player::Runner, Action::Continue);
        pass_until(&mut game, "Approach ICE");
        choose(&mut game, Player::Runner, Action::PassPriority);
        choose(&mut game, Player::Corp, Action::Rez(replacement));
        passes(&mut game);
        assert_eq!(game.observe(Player::Runner).servers[0].ice.len(), 1);
    }
    assert_eq!(game.observe(Player::Corp).corp_credits, 0);
}
