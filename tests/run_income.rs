//! Source-derived #18 cases: CR 26.03; NSG cards/rulings at 6ff8807.
use netrunner_unfolded::game::{
    Action, CardId, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
};

fn choose(game: &mut Match, action: Action) {
    let actor = [Player::Runner, Player::Corp]
        .into_iter()
        .find(|&p| game.observe(p).decision.is_some())
        .unwrap();
    let view = game.observe(actor);
    let d = view.decision.unwrap();
    assert!(
        d.choices.contains(&action),
        "{action:?} unavailable in {}: {:?}",
        view.phase,
        d.choices
    );
    game.choose(actor, d.revision, action).unwrap();
    let resumed = Match::resume(game.checkpoint()).unwrap();
    for p in [Player::Corp, Player::Runner] {
        assert_eq!(game.observe(p), resumed.observe(p));
    }
}
fn until(game: &mut Match, phase: &str) {
    for _ in 0..40 {
        let view = game.observe(Player::Runner);
        if view.phase.starts_with(phase) {
            return;
        }
        choose(
            game,
            if view.phase.starts_with("Movement") {
                Action::Continue
            } else {
                Action::PassPriority
            },
        );
    }
    panic!("did not reach {phase}");
}
fn id(game: &Match, title: &str) -> CardId {
    let v = game.observe(Player::Corp);
    let r = game.observe(Player::Runner);
    v.hand
        .iter()
        .chain(&r.hand)
        .chain(&r.rig)
        .chain(v.servers.iter().flat_map(|s| &s.ice))
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}
fn position(cards: &[(CardKind, Zone, bool)]) -> EngineeringPosition {
    let mut p = EngineeringPosition::economy(18);
    p.active = Player::Runner;
    p.runner_credits = 20;
    p.corp_credits = 10;
    p.shuffle_decks = false;
    p.cards = cards
        .iter()
        .map(|&(kind, zone, rezzed)| EngineeringCard {
            kind,
            zone,
            rezzed,
            advancement: 0,
            installed_turn: matches!(zone, Zone::Rig | Zone::Protecting(_) | Zone::Root(_))
                .then_some(1),
        })
        .collect();
    p
}

#[test]
fn tread_lightly_taxes_later_rez_after_brans_free_inward_install() {
    use CardKind::*;
    let mut game = Match::engineering_turns(position(&[
        (TreadLightly, Zone::Grip, false),
        (Bran, Zone::Protecting(Server::Hq), true),
        (Palisade, Zone::Hq, false),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    let event = id(&game, "Tread Lightly");
    choose(&mut game, Action::PlayCard(event));
    assert_eq!(game.observe(Player::Runner).runner_credits, 19);
    choose(&mut game, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let palisade = id(&game, "Palisade");
    let bran = id(&game, "Brân 1.0");
    choose(&mut game, Action::InstallInward(palisade));
    choose(&mut game, Action::TrashInstalled(bran));
    choose(&mut game, Action::FinishInstall);
    assert_eq!(game.observe(Player::Corp).corp_credits, 10);
    until(&mut game, "Approach");
    choose(&mut game, Action::PassPriority);
    let corp = game.observe(Player::Corp);
    assert!(
        Action::Rez(palisade)
            .description(&corp)
            .contains("6 credits")
    );
    choose(&mut game, Action::Rez(palisade));
    assert_eq!(game.observe(Player::Corp).corp_credits, 4);
    until(&mut game, "Action phase");
    assert!(
        game.observe(Player::Runner)
            .heap
            .iter()
            .any(|c| c.printing == Some("30012"))
    );
}

fn install(game: &mut Match, title: &str) -> CardId {
    until(game, "Action phase");
    let card = game
        .observe(Player::Runner)
        .hand
        .iter()
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap();
    choose(game, Action::InstallCard(card));
    choose(game, Action::FinishInstall);
    card
}

#[test]
fn red_team_loads_each_copy_and_rewards_only_the_source_of_a_successful_run() {
    use CardKind::*;
    let mut game = Match::engineering_turns(position(&[
        (RedTeam, Zone::Grip, false),
        (RedTeam, Zone::Grip, false),
    ]))
    .unwrap();
    let first = install(&mut game, "Red Team");
    let second = install(&mut game, "Red Team");
    until(&mut game, "Action phase");
    assert_eq!(
        game.observe(Player::Runner)
            .rig
            .iter()
            .map(|c| c.hosted_credits)
            .collect::<Vec<_>>(),
        vec![12, 12]
    );
    choose(
        &mut game,
        Action::RunRedTeam {
            card: second,
            server: Server::Hq,
        },
    );
    until(&mut game, "Action phase");
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 13);
    assert_eq!(
        view.rig
            .iter()
            .map(|c| c.hosted_credits)
            .collect::<Vec<_>>(),
        vec![12, 9]
    );
    let choices = view.decision.unwrap().choices;
    assert!(!choices.contains(&Action::RunRedTeam {
        card: first,
        server: Server::Hq
    }));
    assert!(choices.contains(&Action::RunRedTeam {
        card: first,
        server: Server::Rd
    }));
    assert!(!choices.contains(&Action::RunRedTeam {
        card: first,
        server: Server::Remote(1)
    }));
}

#[test]
fn pennyshaver_accumulates_success_only_and_cannot_pay_until_collected() {
    use CardKind::*;
    let mut p = position(&[
        (Pennyshaver, Zone::Grip, false),
        (Cleaver, Zone::Rig, false),
        (Palisade, Zone::Protecting(Server::Hq), true),
    ]);
    p.runner_credits = 3;
    let mut game = Match::engineering_turns(p).unwrap();
    let penny = install(&mut game, "Pennyshaver");
    until(&mut game, "Action phase");
    choose(&mut game, Action::RunServer(Server::Archives));
    until(&mut game, "Action phase");
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 0);
    assert_eq!(
        view.rig
            .iter()
            .find(|c| c.instance == Some(penny))
            .unwrap()
            .hosted_credits,
        1
    );
    choose(&mut game, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .iter()
            .any(|a| matches!(
                a,
                Action::Boost(_) | Action::Break { .. } | Action::PayRunCredits(_)
            ))
    );
    until(&mut game, "Action phase");
    choose(&mut game, Action::TakePennyshaver(penny));
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 2);
    assert_eq!(view.clicks[1], 0);
    assert_eq!(
        view.rig
            .iter()
            .find(|c| c.instance == Some(penny))
            .unwrap()
            .hosted_credits,
        0
    );
}

#[test]
fn pennyshaver_supplies_fifth_memory_and_replacement_discards_old_savings() {
    use CardKind::*;
    let mut game = Match::engineering_turns(position(&[
        (Pennyshaver, Zone::Grip, false),
        (Mayfly, Zone::Rig, false),
        (Mayfly, Zone::Rig, false),
        (Unity, Zone::Grip, false),
        (Pennyshaver, Zone::Grip, false),
    ]))
    .unwrap();
    let old = install(&mut game, "Pennyshaver");
    install(&mut game, "Unity");
    until(&mut game, "Action phase");
    choose(&mut game, Action::RunServer(Server::Archives));
    until(&mut game, "Action phase");
    let new = install(&mut game, "Pennyshaver");
    let view = game.observe(Player::Runner);
    assert_eq!(view.memory_used, 5);
    assert_eq!(view.memory_capacity, 5);
    assert_eq!(
        view.rig
            .iter()
            .filter(|c| c.printing == Some("30014"))
            .count(),
        1
    );
    assert_eq!(
        view.rig
            .iter()
            .find(|c| c.instance == Some(new))
            .unwrap()
            .hosted_credits,
        0
    );
    assert_eq!(
        view.heap
            .iter()
            .find(|c| c.instance == Some(old))
            .unwrap()
            .hosted_credits,
        0
    );
    assert_eq!(view.runner_credits, 11);
}

fn finish_run(game: &mut Match) {
    for _ in 0..80 {
        if game.observe(Player::Runner).run.is_none() {
            return;
        }
        let view = [Player::Runner, Player::Corp]
            .into_iter()
            .map(|p| game.observe(p))
            .find(|v| v.decision.is_some())
            .unwrap();
        let choices = view.decision.unwrap().choices;
        let action = [
            Action::PassPriority,
            Action::Continue,
            Action::FinishAccess,
            Action::AccessHq,
            Action::AccessRd,
        ]
        .into_iter()
        .find(|a| choices.contains(a))
        .unwrap_or(choices[0]);
        choose(game, action);
    }
    panic!("run did not finish");
}

fn next_runner_turn(game: &mut Match) {
    let turn = game.observe(Player::Runner).turn_number;
    for _ in 0..100 {
        let view = game.observe(Player::Runner);
        if view.turn_number > turn
            && view.active_player == Some(Player::Runner)
            && view.phase == "Action phase"
        {
            return;
        }
        choose(
            game,
            if view.phase == "Action phase" {
                Action::GainCredit
            } else {
                Action::PassPriority
            },
        );
    }
    panic!("next Runner turn not reached");
}

#[test]
fn failed_run_before_red_team_install_still_excludes_central_until_next_turn() {
    use CardKind::*;
    let mut game = Match::engineering_turns(position(&[
        (RedTeam, Zone::Grip, false),
        (Palisade, Zone::Protecting(Server::Hq), true),
        (HedgeFund, Zone::Rd, false),
        (HedgeFund, Zone::Rd, false),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Action::RunServer(Server::Hq));
    finish_run(&mut game);
    let red = install(&mut game, "Red Team");
    until(&mut game, "Action phase");
    let illegal = Action::RunRedTeam {
        card: red,
        server: Server::Hq,
    };
    let before = game.observe(Player::Runner);
    assert_eq!(before.central_runs, vec![Server::Hq]);
    assert_eq!(game.observe(Player::Corp).central_runs, before.central_runs);
    let d = before.decision.as_ref().unwrap();
    assert!(!d.choices.contains(&illegal));
    assert!(game.choose(Player::Runner, d.revision, illegal).is_err());
    assert_eq!(before, game.observe(Player::Runner));
    choose(
        &mut game,
        Action::RunRedTeam {
            card: red,
            server: Server::Rd,
        },
    );
    finish_run(&mut game);
    until(&mut game, "Action phase");
    choose(
        &mut game,
        Action::RunRedTeam {
            card: red,
            server: Server::Archives,
        },
    );
    finish_run(&mut game);
    assert_eq!(game.observe(Player::Runner).rig[0].hosted_credits, 6);
    next_runner_turn(&mut game);
    assert!(game.observe(Player::Runner).central_runs.is_empty());
    assert!(
        game.observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&illegal)
    );
    for server in [Server::Rd, Server::Archives] {
        choose(&mut game, Action::RunRedTeam { card: red, server });
        finish_run(&mut game);
        until(&mut game, "Action phase");
    }
    let view = game.observe(Player::Runner);
    assert!(view.rig.is_empty());
    assert_eq!(
        view.heap
            .iter()
            .find(|c| c.instance == Some(red))
            .unwrap()
            .hosted_credits,
        0
    );
    assert_eq!(view.runner_credits, 27);
}

#[test]
fn tread_tax_affects_ice_only_and_expires_after_jacking_out() {
    use CardKind::*;
    let mut p = position(&[
        (TreadLightly, Zone::Grip, false),
        (Palisade, Zone::Protecting(Server::Hq), false),
        (ManegarmSkunkworks, Zone::Root(Server::Hq), false),
    ]);
    p.corp_credits = 5;
    let mut game = Match::engineering_turns(p).unwrap();
    until(&mut game, "Action phase");
    let event = id(&game, "Tread Lightly");
    choose(&mut game, Action::PlayCard(event));
    choose(&mut game, Action::RunServer(Server::Hq));
    until(&mut game, "Approach");
    choose(&mut game, Action::PassPriority);
    let view = game.observe(Player::Corp);
    let ice = id(&game, "Palisade");
    let root = view
        .servers
        .iter()
        .flat_map(|s| &s.root)
        .next()
        .unwrap()
        .instance
        .unwrap();
    assert!(
        !view
            .decision
            .as_ref()
            .unwrap()
            .choices
            .contains(&Action::Rez(ice))
    );
    assert!(Action::Rez(root).description(&view).contains("2 credits"));
    choose(&mut game, Action::Rez(root));
    until(&mut game, "Movement");
    choose(&mut game, Action::JackOut);
    until(&mut game, "Action phase");
    choose(&mut game, Action::RunServer(Server::Hq));
    until(&mut game, "Approach");
    choose(&mut game, Action::PassPriority);
    assert!(
        Action::Rez(ice)
            .description(&game.observe(Player::Corp))
            .contains("3 credits")
    );
    choose(&mut game, Action::Rez(ice));
    assert_eq!(game.observe(Player::Corp).corp_credits, 0);
}

#[test]
fn baseline_uses_public_defenses_and_income_without_hidden_card_knowledge() {
    use CardKind::*;
    let pair = |hidden| {
        let mut game = Match::engineering_turns(position(&[
            (RedTeam, Zone::Rig, false),
            (Pennyshaver, Zone::Rig, false),
            (hidden, Zone::Protecting(Server::Hq), false),
            (HedgeFund, Zone::Hq, false),
        ]))
        .unwrap();
        until(&mut game, "Action phase");
        game
    };
    let mut a = pair(Palisade);
    let b = pair(Bran);
    let av = a.observe(Player::Runner);
    let bv = b.observe(Player::Runner);
    assert_eq!(av, bv);
    let red = id(&a, "Red Team");
    let action = Action::RunRedTeam {
        card: red,
        server: Server::Rd,
    };
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&av),
        Some(action)
    );
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&bv),
        Some(action)
    );
    choose(&mut a, action);
    finish_run(&mut a);
    until(&mut a, "Action phase");
    let action = netrunner_unfolded::game::baseline_response(&a.observe(Player::Runner)).unwrap();
    assert_eq!(
        action,
        Action::RunRedTeam {
            card: red,
            server: Server::Archives
        }
    );
    choose(&mut a, action);
    finish_run(&mut a);
    until(&mut a, "Action phase");
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&a.observe(Player::Runner)),
        Some(Action::TakePennyshaver(id(&a, "Pennyshaver")))
    );
}

#[test]
fn suspended_income_refuses_missing_source_history_and_unsafe_resource_totals() {
    use CardKind::*;
    let mut game = Match::engineering_turns(position(&[
        (RedTeam, Zone::Rig, false),
        (Pennyshaver, Zone::Rig, false),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    let red = id(&game, "Red Team");
    choose(
        &mut game,
        Action::RunRedTeam {
            card: red,
            server: Server::Rd,
        },
    );
    for (pointer, invalid) in [
        ("/state/turn/run/red_team", serde_json::json!(999)),
        ("/state/turn/central_runs", serde_json::json!([])),
        (
            "/state/cards/1/hosted_credits",
            serde_json::json!(u32::MAX - 101),
        ),
    ] {
        let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
        *checkpoint.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            Match::resume(serde_json::from_value(checkpoint).unwrap()).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn send_a_messages_free_rez_ignores_tread_lightly_while_the_run_is_active() {
    use CardKind::*;
    let mut p = position(&[
        (TreadLightly, Zone::Grip, false),
        (SendAMessage, Zone::Rd, false),
        (Palisade, Zone::Protecting(Server::Hq), false),
    ]);
    p.corp_credits = 0;
    let mut game = Match::engineering_turns(p).unwrap();
    until(&mut game, "Action phase");
    let tread = id(&game, "Tread Lightly");
    choose(&mut game, Action::PlayCard(tread));
    choose(&mut game, Action::RunServer(Server::Rd));
    until(&mut game, "Breach R&D");
    choose(&mut game, Action::AccessRd);
    choose(&mut game, Action::StealAgenda);
    let ice = id(&game, "Palisade");
    let view = game.observe(Player::Corp);
    assert!(view.run.as_ref().unwrap().tread_lightly);
    assert!(Action::RezIceFree(ice).description(&view).contains("free"));
    choose(&mut game, Action::RezIceFree(ice));
    let view = game.observe(Player::Corp);
    assert_eq!(view.corp_credits, 0);
    assert!(
        view.servers
            .iter()
            .flat_map(|s| &s.ice)
            .find(|c| c.instance == Some(ice))
            .unwrap()
            .rezzed
    );
}
