//! #19: expectations from CR 26.03 and pinned NSG card text, not engine output.
use netrunner_unfolded::game::{
    Action, CardId, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
};

fn choose(game: &mut Match, action: Action) {
    let view = [Player::Corp, Player::Runner]
        .into_iter()
        .map(|p| game.observe(p))
        .find(|v| v.decision.is_some())
        .unwrap();
    let d = view.decision.unwrap();
    assert!(
        d.choices.contains(&action),
        "{action:?} unavailable in {}: {:?}",
        view.phase,
        d.choices
    );
    game.choose(d.actor, d.revision, action).unwrap();
    let resumed = Match::resume(game.checkpoint()).unwrap();
    for player in [Player::Corp, Player::Runner] {
        assert_eq!(game.observe(player), resumed.observe(player));
    }
}
fn action_phase(game: &mut Match) {
    for _ in 0..20 {
        if game.observe(Player::Runner).phase == "Action phase" {
            return;
        }
        choose(game, Action::PassPriority);
    }
    panic!("No action phase");
}
fn next_turn(game: &mut Match) {
    let number = game.observe(Player::Runner).turn_number;
    for _ in 0..80 {
        if game.observe(Player::Runner).turn_number != number {
            return;
        }
        let view = [Player::Corp, Player::Runner]
            .into_iter()
            .map(|p| game.observe(p))
            .find(|v| v.decision.is_some())
            .unwrap();
        let choices = &view.decision.as_ref().unwrap().choices;
        let action = if choices.contains(&Action::GainCredit) {
            Action::GainCredit
        } else if choices.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            netrunner_unfolded::game::baseline_response(&view).unwrap()
        };
        choose(game, action);
    }
    panic!("Turn did not end");
}
fn position(active: Player, cards: &[(CardKind, Zone)]) -> EngineeringPosition {
    let mut p = EngineeringPosition::economy(19);
    p.active = active;
    p.corp_credits = 10;
    p.runner_credits = 10;
    p.shuffle_decks = false;
    p.cards = cards
        .iter()
        .map(|&(kind, zone)| EngineeringCard::new(kind, zone))
        .collect();
    p.cards
        .extend((0..8).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Rd)));
    p
}
fn id(game: &Match, title: &str) -> CardId {
    let corp = game.observe(Player::Corp);
    let runner = game.observe(Player::Runner);
    corp.hand
        .iter()
        .chain(&runner.hand)
        .chain(&runner.rig)
        .chain(corp.servers.iter().flat_map(|s| &s.root))
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}
fn install(game: &mut Match, title: &str, owner: Player) -> CardId {
    action_phase(game);
    let card = id(game, title);
    choose(game, Action::InstallCard(card));
    if owner == Player::Corp {
        choose(game, Action::Destination(Server::NewRemote));
    }
    choose(game, Action::FinishInstall);
    card
}

#[test]
fn telework_loads_without_gaining_and_resets_each_copy_across_real_turns() {
    let mut game = Match::engineering_turns(position(
        Player::Runner,
        &[
            (CardKind::TeleworkContract, Zone::Grip),
            (CardKind::TeleworkContract, Zone::Grip),
        ],
    ))
    .unwrap();
    let first = install(&mut game, "Telework Contract", Player::Runner);
    let second = install(&mut game, "Telework Contract", Player::Runner);
    action_phase(&mut game);
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 8);
    assert_eq!(
        view.rig
            .iter()
            .map(|c| c.hosted_credits)
            .collect::<Vec<_>>(),
        vec![9, 9]
    );
    choose(&mut game, Action::TakeTelework(first));
    action_phase(&mut game);
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&Action::TakeTelework(first))
    );
    choose(&mut game, Action::TakeTelework(second));
    assert_eq!(game.observe(Player::Runner).runner_credits, 14);
    next_turn(&mut game);
    next_turn(&mut game);
    action_phase(&mut game);
    choose(&mut game, Action::TakeTelework(first));
    next_turn(&mut game);
    next_turn(&mut game);
    action_phase(&mut game);
    choose(&mut game, Action::TakeTelework(first));
    let view = game.observe(Player::Runner);
    assert_eq!(view.rig.len(), 1);
    assert_eq!(view.rig[0].instance, Some(second));
    assert_eq!(view.heap[0].instance, Some(first));
    assert_eq!(view.heap[0].hosted_credits, 0);
}

#[test]
fn regolith_rez_loads_fifteen_then_repeated_clicks_take_three_until_empty() {
    let mut game = Match::engineering_turns(position(
        Player::Corp,
        &[(CardKind::RegolithMiningLicense, Zone::Hq)],
    ))
    .unwrap();
    let card = install(&mut game, "Regolith Mining License", Player::Corp);
    let runner = game.observe(Player::Runner);
    let hidden = &runner
        .servers
        .iter()
        .find(|s| s.server == Server::Remote(1))
        .unwrap()
        .root[0];
    assert_eq!((hidden.printing, hidden.hosted_credits), (None, 0));
    choose(&mut game, Action::Rez(card));
    assert_eq!(game.observe(Player::Corp).corp_credits, 8);
    assert_eq!(
        game.observe(Player::Runner).servers[3].root[0].hosted_credits,
        15
    );
    for _ in 0..2 {
        action_phase(&mut game);
        choose(&mut game, Action::TakeRegolith(card));
    }
    assert_eq!(game.observe(Player::Corp).corp_credits, 14);
    next_turn(&mut game);
    next_turn(&mut game);
    for _ in 0..3 {
        action_phase(&mut game);
        choose(&mut game, Action::TakeRegolith(card));
    }
    let view = game.observe(Player::Corp);
    assert_eq!(view.corp_credits, 23);
    assert!(
        !view
            .servers
            .iter()
            .flat_map(|s| &s.root)
            .any(|c| c.instance == Some(card))
    );
    assert!(
        view.archives
            .iter()
            .any(|c| c.instance == Some(card) && c.face_up && c.hosted_credits == 0)
    );
}

#[test]
fn nico_pays_only_at_formal_turn_start_and_final_payout_draws_before_mandatory_draw() {
    let mut game = Match::engineering_turns(position(
        Player::Corp,
        &[(CardKind::NicoCampaign, Zone::Hq)],
    ))
    .unwrap();
    let card = install(&mut game, "Nico Campaign", Player::Corp);
    choose(&mut game, Action::Rez(card));
    assert_eq!(game.observe(Player::Corp).corp_credits, 8);
    assert_eq!(
        game.observe(Player::Runner).servers[3].root[0].hosted_credits,
        9
    );
    action_phase(&mut game);
    assert_eq!(game.observe(Player::Corp).corp_credits, 8); // Rezzed after this turn began.
    for remaining in [6, 3, 0] {
        next_turn(&mut game);
        next_turn(&mut game);
        let before = game.observe(Player::Corp);
        action_phase(&mut game);
        let after = game.observe(Player::Corp);
        assert_eq!(after.corp_credits, before.corp_credits + 3);
        assert_eq!(
            after.hq_count,
            before.hq_count + if remaining == 0 { 2 } else { 1 }
        );
        if remaining > 0 {
            assert_eq!(after.servers[3].root[0].hosted_credits, remaining);
        } else {
            assert!(
                after
                    .archives
                    .iter()
                    .any(|c| c.instance == Some(card) && c.face_up)
            );
        }
    }
}

#[test]
fn smartware_places_from_bank_without_gaining_and_automatically_takes_one_each_runner_turn() {
    let mut game = Match::engineering_turns(position(
        Player::Runner,
        &[(CardKind::SmartwareDistributor, Zone::Grip)],
    ))
    .unwrap();
    let card = install(&mut game, "Smartware Distributor", Player::Runner);
    assert_eq!(game.observe(Player::Runner).rig[0].hosted_credits, 0);
    for _ in 0..2 {
        action_phase(&mut game);
        choose(&mut game, Action::LoadSmartware(card));
    }
    let view = game.observe(Player::Runner);
    assert_eq!((view.runner_credits, view.rig[0].hosted_credits), (10, 6));
    for (prior, remaining) in [(6, 5), (5, 4), (4, 3), (3, 2), (2, 1), (1, 0), (0, 0)] {
        next_turn(&mut game);
        action_phase(&mut game);
        assert_eq!(game.observe(Player::Runner).rig[0].hosted_credits, prior);
        next_turn(&mut game);
        let before = game.observe(Player::Runner);
        action_phase(&mut game);
        let after = game.observe(Player::Runner);
        assert_eq!(after.rig[0].hosted_credits, remaining);
        assert_eq!(
            after.runner_credits,
            before.runner_credits + u32::from(before.rig[0].hosted_credits > 0)
        );
    }
}

#[test]
fn simultaneous_campaigns_offer_only_ordering_then_resume_without_duplicate_payouts() {
    let mut p = position(Player::Corp, &[]);
    for remote in [1, 2] {
        p.cards.push(EngineeringCard {
            kind: CardKind::NicoCampaign,
            zone: Zone::Root(Server::Remote(remote)),
            rezzed: true,
            advancement: 0,
            installed_turn: Some(1),
        });
    }
    let mut game = Match::engineering_turns(p).unwrap();
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let before = game.observe(Player::Corp);
    assert_eq!(before.phase, "Turn begins · order income abilities");
    let first = before.servers[3].root[0].instance.unwrap();
    let second = before.servers[4].root[0].instance.unwrap();
    assert_eq!(
        before.decision.unwrap().choices,
        vec![Action::ResolveIncome(first), Action::ResolveIncome(second)]
    );
    assert_eq!(before.hq_count, 0);
    choose(&mut game, Action::ResolveIncome(second));
    let after = game.observe(Player::Corp);
    assert_eq!(after.corp_credits, 16);
    assert_eq!(after.hq_count, 1);
    assert_eq!(after.servers[3].root[0].hosted_credits, 6);
    assert_eq!(after.servers[4].root[0].hosted_credits, 6);
    assert_eq!(after.phase, "Paid / rez / scoring window");
}

#[test]
fn runner_can_pay_correct_trash_costs_without_triggering_campaign_depletion_draw() {
    for (kind, cost) in [
        (CardKind::NicoCampaign, 2),
        (CardKind::RegolithMiningLicense, 3),
    ] {
        let mut p = position(Player::Runner, &[]);
        p.cards.push(EngineeringCard {
            kind,
            zone: Zone::Root(Server::Remote(1)),
            rezzed: true,
            advancement: 0,
            installed_turn: Some(1),
        });
        let mut game = Match::engineering_turns(p).unwrap();
        action_phase(&mut game);
        choose(&mut game, Action::RunServer(Server::Remote(1)));
        for _ in 0..30 {
            let view = [Player::Runner, Player::Corp]
                .into_iter()
                .map(|p| game.observe(p))
                .find(|v| v.decision.is_some())
                .unwrap();
            if view.accessed.is_some() {
                break;
            }
            let choices = &view.decision.as_ref().unwrap().choices;
            choose(
                &mut game,
                if choices.contains(&Action::AccessRoot(0)) {
                    Action::AccessRoot(0)
                } else if choices.contains(&Action::Continue) {
                    Action::Continue
                } else {
                    Action::PassPriority
                },
            );
        }
        let view = game.observe(Player::Runner);
        assert!(
            view.phase
                .starts_with(&format!("Accessing {}", kind.description().0)),
            "{}",
            view.phase
        );
        assert!(
            Action::TrashAccessed
                .description(&view)
                .contains(&format!("{cost} credits"))
        );
        choose(&mut game, Action::TrashAccessed);
        choose(&mut game, Action::PayRunCredits(0));
        let after = game.observe(Player::Runner);
        assert_eq!(after.runner_credits, 10 - cost);
        assert_eq!(after.hq_count, 0);
        assert_eq!(after.archives[0].hosted_credits, 0);
        assert!(after.archives[0].face_up);
        assert!(!after.history.iter().any(|line| line.contains("is empty")));
    }
}

#[test]
fn economic_baseline_uses_only_its_observation_and_hidden_campaigns_remain_indistinguishable() {
    let mut runner_views = Vec::new();
    for kind in [CardKind::NicoCampaign, CardKind::RegolithMiningLicense] {
        let mut game =
            Match::engineering_turns(position(Player::Corp, &[(kind, Zone::Hq)])).unwrap();
        install(&mut game, kind.description().0, Player::Corp);
        runner_views.push(game.observe(Player::Runner));
    }
    assert_eq!(runner_views[0], runner_views[1]);
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&runner_views[0]),
        netrunner_unfolded::game::baseline_response(&runner_views[1])
    );
    let mut game = Match::engineering_turns(position(
        Player::Runner,
        &[
            (CardKind::TeleworkContract, Zone::Grip),
            (CardKind::SmartwareDistributor, Zone::Grip),
        ],
    ))
    .unwrap();
    let telework = install(&mut game, "Telework Contract", Player::Runner);
    action_phase(&mut game);
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&game.observe(Player::Runner)),
        Some(Action::TakeTelework(telework))
    );
    choose(&mut game, Action::TakeTelework(telework));
    let smartware = install(&mut game, "Smartware Distributor", Player::Runner);
    action_phase(&mut game);
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&game.observe(Player::Runner)),
        Some(Action::LoadSmartware(smartware))
    );
}

#[test]
fn final_nico_draw_can_end_match_before_other_income_or_mandatory_draw() {
    let mut p = position(Player::Corp, &[]);
    p.cards.truncate(2); // Exactly two mandatory draws before the third payout.
    for remote in [1, 2] {
        p.cards.push(EngineeringCard {
            kind: CardKind::NicoCampaign,
            zone: Zone::Root(Server::Remote(remote)),
            rezzed: true,
            advancement: 0,
            installed_turn: Some(1),
        });
    }
    let mut game = Match::engineering_turns(p).unwrap();
    for turn in [2, 4] {
        assert_eq!(game.observe(Player::Corp).turn_number, turn);
        choose(&mut game, Action::PassPriority);
        choose(&mut game, Action::PassPriority);
        let card = game.observe(Player::Corp).servers[3].root[0]
            .instance
            .unwrap();
        choose(&mut game, Action::ResolveIncome(card));
        next_turn(&mut game);
        next_turn(&mut game);
    }
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let before = game.observe(Player::Corp);
    let second = before.servers[4].root[0].instance.unwrap();
    choose(&mut game, Action::ResolveIncome(second));
    let after = game.observe(Player::Corp);
    assert_eq!(after.match_winner, Some(Player::Runner));
    assert_eq!(after.corp_credits, before.corp_credits + 3);
    assert_eq!(after.archives[0].instance, Some(second));
    assert_eq!(after.servers[3].root[0].hosted_credits, 3);
    assert_eq!(
        after
            .history
            .iter()
            .filter(|line| line.contains("cannot draw"))
            .count(),
        1
    );
}

#[test]
fn declining_prebegin_rez_keeps_campaign_hidden_and_cannot_pay_retroactively() {
    let mut p = position(Player::Corp, &[]);
    p.cards.push(EngineeringCard {
        kind: CardKind::NicoCampaign,
        zone: Zone::Root(Server::Remote(1)),
        rezzed: false,
        advancement: 0,
        installed_turn: Some(1),
    });
    let mut game = Match::engineering_turns(p).unwrap();
    let card = id(&game, "Nico Campaign");
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let before = game.observe(Player::Runner);
    assert_eq!(before.corp_credits, 10);
    assert_eq!(before.servers[3].root[0].hosted_credits, 0);
    assert_eq!(before.servers[3].root[0].printing, None);
    choose(&mut game, Action::Rez(card));
    action_phase(&mut game);
    let after = game.observe(Player::Runner);
    assert_eq!(after.corp_credits, 8);
    assert_eq!(after.servers[3].root[0].hosted_credits, 9);
}

#[test]
fn each_smartware_copy_resolves_once_in_the_selected_order() {
    let mut game = Match::engineering_turns(position(
        Player::Runner,
        &[
            (CardKind::SmartwareDistributor, Zone::Grip),
            (CardKind::SmartwareDistributor, Zone::Grip),
        ],
    ))
    .unwrap();
    let first = install(&mut game, "Smartware Distributor", Player::Runner);
    let second = install(&mut game, "Smartware Distributor", Player::Runner);
    for card in [first, second] {
        action_phase(&mut game);
        choose(&mut game, Action::LoadSmartware(card));
    }
    next_turn(&mut game);
    next_turn(&mut game);
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let before = game.observe(Player::Runner);
    assert_eq!(
        before.decision.unwrap().choices,
        vec![Action::ResolveIncome(first), Action::ResolveIncome(second)]
    );
    choose(&mut game, Action::ResolveIncome(second));
    let after = game.observe(Player::Runner);
    assert_eq!(after.runner_credits, before.runner_credits + 2);
    assert_eq!(
        after
            .rig
            .iter()
            .map(|c| c.hosted_credits)
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
}

#[test]
fn save_admission_rejects_duplicate_income_sources_and_foreign_usage_limits() {
    let mut p = position(Player::Corp, &[]);
    for remote in [1, 2] {
        p.cards.push(EngineeringCard {
            kind: CardKind::NicoCampaign,
            zone: Zone::Root(Server::Remote(remote)),
            rezzed: true,
            advancement: 0,
            installed_turn: Some(1),
        });
    }
    let mut game = Match::engineering_turns(p).unwrap();
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let good = serde_json::to_value(game.checkpoint()).unwrap();
    let mut duplicate = good.clone();
    duplicate["state"]["turn"]["income_pending"][1] =
        duplicate["state"]["turn"]["income_pending"][0].clone();
    assert!(Match::resume(serde_json::from_value(duplicate).unwrap()).is_err());
    let mut foreign = good;
    foreign["state"]["cards"][0]["telework_used_turn"] = serde_json::json!(2);
    assert!(Match::resume(serde_json::from_value(foreign).unwrap()).is_err());
}
