//! Independent expectations: CR 26.03 3.9.5b, 6.9.5a, 9.6.13, 9.10.4;
//! NSG card data 6ff8807; NRDB verified Mayfly ruling, refreshed 2026-11-14.
use netrunner_unfolded::game::{
    Action, CardId, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
};

fn choose(game: &mut Match, player: Player, action: Action) {
    let view = game.observe(player);
    let d = view.decision.expect("actor has decision");
    assert!(
        d.choices.contains(&action),
        "{action:?} unavailable in {}: {:?}",
        view.phase,
        d.choices
    );
    game.choose(player, d.revision, action).unwrap();
    let resumed = Match::resume(game.checkpoint()).unwrap();
    for viewer in [Player::Corp, Player::Runner] {
        assert_eq!(game.observe(viewer), resumed.observe(viewer));
    }
}
fn pass(game: &mut Match) {
    let actor = [Player::Runner, Player::Corp]
        .into_iter()
        .find(|&p| game.observe(p).decision.is_some())
        .unwrap();
    choose(game, actor, Action::PassPriority);
}
fn until(game: &mut Match, phase: &str) {
    for _ in 0..32 {
        if game.observe(Player::Runner).phase.starts_with(phase) {
            return;
        }
        if game.observe(Player::Runner).phase.starts_with("Movement") {
            choose(game, Player::Runner, Action::Continue);
            continue;
        }
        pass(game);
    }
    panic!("did not reach {phase}");
}
fn id(game: &Match, title: &str) -> CardId {
    let v = game.observe(Player::Runner);
    v.hand
        .iter()
        .chain(&v.rig)
        .chain(&v.heap)
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}
fn position(cards: &[(CardKind, Zone)]) -> EngineeringPosition {
    let mut p = EngineeringPosition::economy(17);
    p.active = Player::Runner;
    p.runner_credits = 20;
    p.corp_credits = 0;
    p.shuffle_decks = false;
    p.cards = cards
        .iter()
        .map(|&(kind, zone)| EngineeringCard {
            kind,
            zone,
            advancement: 0,
            installed_turn: matches!(zone, Zone::Rig | Zone::Protecting(_)).then_some(1),
            rezzed: matches!(zone, Zone::Protecting(_)),
        })
        .collect();
    p
}
fn install(game: &mut Match, title: &str) {
    until(game, "Action phase");
    choose(game, Player::Runner, Action::InstallCard(id(game, title)));
    choose(game, Player::Runner, Action::FinishInstall);
}

#[test]
fn carmen_install_discount_is_earned_by_success_and_survives_a_later_failed_run() {
    let mut p = position(&[
        (CardKind::Carmen, Zone::Grip),
        (CardKind::Palisade, Zone::Protecting(Server::Hq)),
    ]);
    p.runner_credits = 3;
    let mut game = Match::engineering_turns(p).unwrap();
    until(&mut game, "Action phase");
    let carmen = id(&game, "Carmen");
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&Action::InstallCard(carmen))
    );
    choose(
        &mut game,
        Player::Runner,
        Action::RunServer(Server::Archives),
    );
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Action phase");
    install(&mut game, "Carmen");
    let view = game.observe(Player::Runner);
    assert_eq!(view.runner_credits, 0);
    assert_eq!(view.rig[0].title.as_deref(), Some("Carmen"));
}

#[test]
fn carmen_breaks_karuna_with_chosen_payments_but_not_inner_code_gate() {
    let mut game = Match::engineering_turns(position(&[
        (CardKind::Carmen, Zone::Rig),
        (CardKind::Overclock, Zone::Grip),
        (CardKind::Diviner, Zone::Protecting(Server::Hq)),
        (CardKind::Karuna, Zone::Protecting(Server::Hq)),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    let event = id(&game, "Overclock");
    choose(&mut game, Player::Runner, Action::PlayCard(event));
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    let carmen = id(&game, "Carmen");
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .iter()
            .any(|a| matches!(a, Action::Break { .. }))
    );
    choose(&mut game, Player::Runner, Action::Boost(carmen));
    assert_eq!(
        game.observe(Player::Runner).decision.unwrap().choices,
        vec![
            Action::PayRunCredits(0),
            Action::PayRunCredits(1),
            Action::PayRunCredits(2)
        ]
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(1));
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[0].2, 5);
    for (mask, hosted) in [(1, 0), (2, 1)] {
        choose(
            &mut game,
            Player::Runner,
            Action::Break {
                breaker: carmen,
                subroutines: mask,
            },
        );
        choose(&mut game, Player::Runner, Action::PayRunCredits(hosted));
    }
    until(&mut game, "Movement");
    assert_eq!(game.observe(Player::Runner).runner_credits, 17);
    assert_eq!(game.observe(Player::Runner).run.unwrap().hosted_credits, 3);
    choose(&mut game, Player::Runner, Action::Continue);
    until(&mut game, "Encounter");
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[0].2, 2);
    choose(&mut game, Player::Runner, Action::Boost(carmen));
    choose(&mut game, Player::Runner, Action::PayRunCredits(2));
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .iter()
            .any(|a| matches!(a, Action::Break { .. }))
    );
}

#[test]
fn mayfly_crosses_ice_subtypes_then_trashes_once_at_run_end() {
    let mut game = Match::engineering_turns(position(&[
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Carmen, Zone::Rig),
        (CardKind::Unity, Zone::Rig),
        (CardKind::Whitespace, Zone::Protecting(Server::Hq)),
        (CardKind::Palisade, Zone::Protecting(Server::Hq)),
        (CardKind::Tithe, Zone::Protecting(Server::Hq)),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    let mayfly = id(&game, "Mayfly");
    let unity = id(&game, "Unity");
    choose(&mut game, Player::Runner, Action::Boost(unity));
    choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[2].2, 4);
    for (index, masks) in [vec![1, 2], vec![1], vec![1, 2]].into_iter().enumerate() {
        if index == 1 {
            choose(&mut game, Player::Runner, Action::Boost(mayfly));
            choose(&mut game, Player::Runner, Action::PayRunCredits(0));
        }
        for mask in masks {
            choose(
                &mut game,
                Player::Runner,
                Action::Break {
                    breaker: mayfly,
                    subroutines: mask,
                },
            );
            choose(&mut game, Player::Runner, Action::PayRunCredits(0));
        }
        until(&mut game, "Movement");
        assert!(
            game.observe(Player::Runner)
                .rig
                .iter()
                .any(|c| c.instance == Some(mayfly))
        );
        assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[0].2, 1);
        choose(&mut game, Player::Runner, Action::Continue);
        if index < 2 {
            until(&mut game, "Encounter");
        }
    }
    until(&mut game, "Action phase");
    let v = game.observe(Player::Runner);
    assert_eq!(v.rig.len(), 2);
    assert_eq!(
        v.heap
            .iter()
            .filter(|c| c.title.as_deref() == Some("Mayfly"))
            .count(),
        1
    );
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    choose(&mut game, Player::Runner, Action::Boost(unity));
    choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[1].2, 3);
}

#[test]
fn mayfly_boost_alone_survives_but_breaking_trashes_on_jackout_or_ice_end() {
    for ending in ["boost-only", "jack-out", "ice-end", "karuna"] {
        let mut cards = vec![(CardKind::Mayfly, Zone::Rig)];
        cards.push((
            if ending == "karuna" {
                CardKind::Karuna
            } else {
                CardKind::Bran
            },
            Zone::Protecting(Server::Hq),
        ));
        cards.extend([(CardKind::SureGamble, Zone::Grip); 3]);
        let mut game = Match::engineering_turns(position(&cards)).unwrap();
        until(&mut game, "Action phase");
        choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
        until(&mut game, "Encounter");
        let mayfly = id(&game, "Mayfly");
        let boosts = if ending == "karuna" { 2 } else { 5 };
        for _ in 0..boosts {
            choose(&mut game, Player::Runner, Action::Boost(mayfly));
            choose(&mut game, Player::Runner, Action::PayRunCredits(0));
        }
        if ending != "boost-only" {
            choose(
                &mut game,
                Player::Runner,
                Action::Break {
                    breaker: mayfly,
                    subroutines: if ending == "karuna" { 2 } else { 1 },
                },
            );
            choose(&mut game, Player::Runner, Action::PayRunCredits(0));
        }
        if ending == "jack-out" {
            for mask in [2, 4] {
                choose(
                    &mut game,
                    Player::Runner,
                    Action::Break {
                        breaker: mayfly,
                        subroutines: mask,
                    },
                );
                choose(&mut game, Player::Runner, Action::PayRunCredits(0));
            }
            until(&mut game, "Movement");
            choose(&mut game, Player::Runner, Action::JackOut);
        } else if ending == "karuna" {
            until(&mut game, "Karunā");
            choose(&mut game, Player::Runner, Action::JackOut);
        } else {
            pass(&mut game);
            pass(&mut game);
            if ending == "boost-only" {
                choose(&mut game, Player::Corp, Action::DeclineInstall);
            }
        }
        assert!(game.observe(Player::Runner).run.is_none());
        assert_eq!(
            game.observe(Player::Runner).rig.len(),
            usize::from(ending == "boost-only")
        );
    }
}

#[test]
fn two_mu_mayfly_install_requires_chosen_trash_and_both_new_breakers_are_nonunique() {
    let mut game = Match::engineering_turns(position(&[
        (CardKind::Unity, Zone::Rig),
        (CardKind::Carmen, Zone::Rig),
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Mayfly, Zone::Grip),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    let new = id(&game, "Mayfly");
    choose(&mut game, Player::Runner, Action::InstallCard(new));
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&Action::FinishInstall)
    );
    let carmen = id(&game, "Carmen");
    let unity = id(&game, "Unity");
    choose(&mut game, Player::Runner, Action::TrashInstalled(carmen));
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&Action::FinishInstall)
    );
    choose(&mut game, Player::Runner, Action::TrashInstalled(unity));
    choose(&mut game, Player::Runner, Action::FinishInstall);
    assert_eq!(game.observe(Player::Runner).memory_used, 4);
    assert_eq!(game.observe(Player::Runner).rig.len(), 2);
    let mut game = Match::engineering_turns(position(&[
        (CardKind::Carmen, Zone::Rig),
        (CardKind::Carmen, Zone::Grip),
    ]))
    .unwrap();
    install(&mut game, "Carmen");
    assert_eq!(game.observe(Player::Runner).rig.len(), 2);
    assert_eq!(game.observe(Player::Runner).runner_credits, 15);
}

#[test]
fn carmen_discount_expires_at_turn_change() {
    let mut p = position(&[
        (CardKind::Carmen, Zone::Grip),
        (CardKind::HedgeFund, Zone::Rd),
    ]);
    p.runner_credits = 3;
    let mut game = Match::engineering_turns(p).unwrap();
    until(&mut game, "Action phase");
    choose(
        &mut game,
        Player::Runner,
        Action::RunServer(Server::Archives),
    );
    until(&mut game, "Action phase");
    // Use the three remaining clicks on harmless empty Archives runs, avoiding
    // credit gain that could hide whether the discount expired.
    for _ in 0..3 {
        choose(
            &mut game,
            Player::Runner,
            Action::RunServer(Server::Archives),
        );
        let phase = if game.observe(Player::Runner).clicks[1] == 0 {
            "Turn end"
        } else {
            "Action phase"
        };
        until(&mut game, phase);
    }
    until(&mut game, "Action phase");
    choose(&mut game, Player::Corp, Action::Purge);
    until(&mut game, "Action phase");
    let carmen = id(&game, "Carmen");
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&Action::InstallCard(carmen))
    );
}

#[test]
fn baseline_boosts_carmen_against_sentry_and_mayfly_against_barrier() {
    for (breaker, ice) in [
        (CardKind::Carmen, CardKind::Karuna),
        (CardKind::Mayfly, CardKind::Palisade),
    ] {
        let mut game = Match::engineering_turns(position(&[
            (breaker, Zone::Rig),
            (ice, Zone::Protecting(Server::Hq)),
        ]))
        .unwrap();
        until(&mut game, "Action phase");
        choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
        until(&mut game, "Encounter");
        let view = game.observe(Player::Runner);
        assert!(matches!(
            netrunner_unfolded::game::baseline_response(&view),
            Some(Action::Boost(_))
        ));
    }
}

#[test]
fn choices_explain_cost_duration_rig_count_and_mayfly_delayed_trash() {
    let mut game = Match::engineering_turns(position(&[
        (CardKind::Carmen, Zone::Rig),
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Unity, Zone::Rig),
        (CardKind::Tithe, Zone::Protecting(Server::Hq)),
    ]))
    .unwrap();
    let carmen = id(&game, "Carmen");
    let mayfly = id(&game, "Mayfly");
    let unity = id(&game, "Unity");
    let view = game.observe(Player::Runner);
    assert_eq!(
        Action::Boost(carmen).description(&view),
        "Carmen · +3 strength · 2 credits · expires immediately"
    );
    assert_eq!(
        Action::Boost(mayfly).description(&view),
        "Mayfly · +1 strength · 1 credit · expires immediately"
    );
    assert_eq!(
        Action::Boost(unity).description(&view),
        "Unity · +3 strength · 1 credit · expires immediately"
    );
    choose(&mut game, Player::Runner, Action::Boost(carmen));
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    assert_eq!(game.observe(Player::Runner).run.unwrap().breakers[0].2, 2);
    let view = game.observe(Player::Runner);
    assert_eq!(
        Action::Boost(carmen).description(&view),
        "Carmen · +3 strength · 2 credits · this encounter"
    );
    assert!(
        Action::Break {
            breaker: mayfly,
            subroutines: 1
        }
        .description(&view)
        .contains("trash at run end")
    );
}

#[test]
fn baseline_uses_an_affordable_ai_boost_when_the_killer_boost_is_unaffordable() {
    let mut p = position(&[
        (CardKind::Carmen, Zone::Rig),
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Karuna, Zone::Protecting(Server::Hq)),
    ]);
    p.runner_credits = 1;
    let mut game = Match::engineering_turns(p).unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&game.observe(Player::Runner)),
        Some(Action::Boost(id(&game, "Mayfly")))
    );
}

#[test]
fn delayed_trash_is_paid_and_instance_specific_and_malformed_sources_are_refused() {
    let mut game = Match::engineering_turns(position(&[
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Tithe, Zone::Protecting(Server::Hq)),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    let view = game.observe(Player::Runner);
    let copies: Vec<_> = view.rig.iter().map(|c| c.instance.unwrap()).collect();
    assert_ne!(
        Action::Break {
            breaker: copies[0],
            subroutines: 1
        }
        .description(&view),
        Action::Break {
            breaker: copies[1],
            subroutines: 1
        }
        .description(&view)
    );
    assert_ne!(
        Action::Boost(copies[0]).description(&view),
        Action::Boost(copies[1]).description(&view)
    );
    let mayfly = id(&game, "Mayfly");
    choose(
        &mut game,
        Player::Runner,
        Action::Break {
            breaker: mayfly,
            subroutines: 1,
        },
    );
    assert_eq!(
        game.observe(Player::Runner)
            .run
            .unwrap()
            .payment_for
            .as_deref(),
        Some("Mayfly · rig 1: break subroutine 1")
    );
    assert!(
        game.observe(Player::Runner)
            .run
            .unwrap()
            .trash_at_run_end
            .is_empty()
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    assert_eq!(
        game.observe(Player::Runner).run.unwrap().trash_at_run_end,
        vec![mayfly]
    );
    for malformed in [
        serde_json::json!([999]),
        serde_json::json!([mayfly, mayfly]),
    ] {
        let mut saved = serde_json::to_value(game.checkpoint()).unwrap();
        saved["state"]["turn"]["run"]["trash_at_run_end"] = malformed;
        assert!(Match::resume(serde_json::from_value(saved).unwrap()).is_err());
    }
    until(&mut game, "Movement");
    choose(&mut game, Player::Runner, Action::JackOut);
    assert_eq!(game.observe(Player::Runner).rig.len(), 1);
    assert_ne!(game.observe(Player::Runner).rig[0].instance, Some(mayfly));
    assert_eq!(game.observe(Player::Runner).memory_used, 2);
}

#[test]
fn mayfly_trashes_on_manegarm_refusal_but_game_end_stops_delayed_resolution() {
    let mut p = position(&[
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Tithe, Zone::Protecting(Server::Hq)),
        (CardKind::ManegarmSkunkworks, Zone::Root(Server::Hq)),
    ]);
    p.cards.last_mut().unwrap().installed_turn = Some(1);
    p.cards.last_mut().unwrap().rezzed = true;
    let mut game = Match::engineering_turns(p).unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    let mayfly = id(&game, "Mayfly");
    choose(
        &mut game,
        Player::Runner,
        Action::Break {
            breaker: mayfly,
            subroutines: 1,
        },
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    until(&mut game, "Manegarm");
    choose(&mut game, Player::Runner, Action::RefuseManegarm);
    assert!(game.observe(Player::Runner).rig.is_empty());

    let mut game = Match::engineering_turns(position(&[
        (CardKind::Mayfly, Zone::Rig),
        (CardKind::Karuna, Zone::Protecting(Server::Hq)),
        (CardKind::SureGamble, Zone::Grip),
    ]))
    .unwrap();
    until(&mut game, "Action phase");
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    until(&mut game, "Encounter");
    let mayfly = id(&game, "Mayfly");
    for _ in 0..2 {
        choose(&mut game, Player::Runner, Action::Boost(mayfly));
        choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    }
    choose(
        &mut game,
        Player::Runner,
        Action::Break {
            breaker: mayfly,
            subroutines: 2,
        },
    );
    choose(&mut game, Player::Runner, Action::PayRunCredits(0));
    pass(&mut game);
    pass(&mut game);
    let view = game.observe(Player::Runner);
    assert_eq!(view.match_winner, Some(Player::Corp));
    assert_eq!(view.rig.len(), 1);
    assert!(view.run_result.is_none());
}
