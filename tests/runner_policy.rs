//! Worked source expectations, not selector-score snapshots. CR26.03 and the
//! pinned NSG introductory card text govern each arithmetic/choice case.
use netrunner_unfolded::{
    game::{
        Action, CardId, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
    },
    policies::Policy,
};
fn choose(g: &mut Match, a: Action) {
    let actor = [Player::Runner, Player::Corp]
        .into_iter()
        .find(|p| g.observe(*p).decision.is_some())
        .unwrap();
    let d = g.observe(actor).decision.unwrap();
    assert!(d.choices.contains(&a), "{a:?} unavailable: {:?}", d.choices);
    g.choose(actor, d.revision, a).unwrap();
}
fn action_window(g: &mut Match) {
    for _ in 0..12 {
        if g.observe(Player::Runner)
            .decision
            .is_some_and(|d| d.choices.contains(&Action::GainCredit))
        {
            return;
        }
        choose(g, Action::PassPriority);
    }
    panic!("no action window");
}
fn position(cards: &[(CardKind, Zone)], credits: u32) -> Match {
    let mut p = EngineeringPosition::economy(3);
    p.active = Player::Runner;
    p.runner_credits = credits;
    p.corp_credits = 0;
    p.shuffle_decks = false;
    p.cards = cards
        .iter()
        .map(|&(kind, zone)| EngineeringCard {
            kind,
            zone,
            advancement: 0,
            installed_turn: matches!(zone, Zone::Rig | Zone::Protecting(_) | Zone::Root(_))
                .then_some(1),
            rezzed: matches!(zone, Zone::Protecting(_)),
        })
        .collect();
    Match::engineering_turns(p).unwrap()
}
fn id(g: &Match, printing: &str) -> CardId {
    let v = g.observe(Player::Runner);
    v.hand
        .iter()
        .chain(&v.rig)
        .find(|c| c.printing == Some(printing))
        .unwrap()
        .instance
        .unwrap()
}
#[test]
fn missing_sentry_coverage_is_installed_before_a_redundant_barrier_breaker() {
    let mut g = position(
        &[
            (CardKind::Cleaver, Zone::Rig),
            (CardKind::Cleaver, Zone::Grip),
            (CardKind::Carmen, Zone::Grip),
            (CardKind::Tithe, Zone::Protecting(Server::Hq)),
            (CardKind::SureGamble, Zone::Grip),
            (CardKind::SureGamble, Zone::Stack),
        ],
        8,
    );
    action_window(&mut g);
    // Carmen covers the exposed sentry; another Cleaver cannot interact with it.
    assert_eq!(
        Policy::RunnerV1.choose(&g.observe(Player::Runner)),
        Some(Action::InstallCard(id(&g, "30015")))
    );
}
fn encounter(g: &mut Match, server: Server) {
    action_window(g);
    choose(g, Action::RunServer(server));
    for _ in 0..30 {
        if g.observe(Player::Runner)
            .run
            .as_ref()
            .is_some_and(|r| r.encounter_paid)
        {
            return;
        }
        let v = g.observe(Player::Runner);
        if v.decision
            .as_ref()
            .is_some_and(|d| d.choices.contains(&Action::Continue))
        {
            choose(g, Action::Continue);
        } else {
            choose(g, Action::PassPriority);
        }
    }
    panic!("no encounter");
}
#[test]
fn carmen_complete_four_credit_karuna_line_preserves_mayfly() {
    let mut g = position(
        &[
            (CardKind::Carmen, Zone::Rig),
            (CardKind::Mayfly, Zone::Rig),
            (CardKind::Karuna, Zone::Protecting(Server::Hq)),
            (CardKind::SureGamble, Zone::Grip),
        ],
        4,
    );
    encounter(&mut g, Server::Hq);
    let carmen = id(&g, "30015");
    // Carmen +3 costs two; two one-credit breaks prevent four lethal damage.
    // Mayfly costs the same four credits but is lost at run end.
    for expected in [
        Action::Boost(carmen),
        Action::PayRunCredits(0),
        Action::Break {
            breaker: carmen,
            subroutines: 1,
        },
        Action::PayRunCredits(0),
        Action::Break {
            breaker: carmen,
            subroutines: 2,
        },
        Action::PayRunCredits(0),
        Action::PassPriority,
    ] {
        assert_eq!(
            Policy::RunnerV1.choose(&g.observe(Player::Runner)),
            Some(expected)
        );
        choose(&mut g, expected);
    }
    assert_eq!(g.observe(Player::Runner).runner_credits, 0);
    assert!(
        g.observe(Player::Runner)
            .run
            .unwrap()
            .trash_at_run_end
            .is_empty()
    );
}
#[test]
fn an_affordable_first_break_is_declined_when_the_known_rest_cannot_be_paid() {
    for manegarm in [false, true] {
        let mut cards = vec![
            (CardKind::Cleaver, Zone::Rig),
            (CardKind::SureGamble, Zone::Grip),
        ];
        if manegarm {
            cards.push((CardKind::ManegarmSkunkworks, Zone::Root(Server::Hq)));
        } else {
            cards.push((CardKind::Bran, Zone::Protecting(Server::Hq)));
        }
        cards.push((CardKind::Palisade, Zone::Protecting(Server::Hq)));
        let mut g = position(&cards, 1);
        if manegarm {
            // Expose the upgrade legally, with no extra pool left.
            let mut p = EngineeringPosition::economy(3);
            p.active = Player::Runner;
            p.corp_credits = 0;
            p.runner_credits = 1;
            p.shuffle_decks = false;
            p.cards = cards
                .iter()
                .map(|&(kind, zone)| EngineeringCard {
                    kind,
                    zone,
                    advancement: 0,
                    installed_turn: Some(1),
                    rezzed: matches!(zone, Zone::Protecting(_) | Zone::Root(_)),
                })
                .collect();
            g = Match::engineering_turns(p).unwrap();
        }
        action_window(&mut g);
        choose(&mut g, Action::GainCredit);
        action_window(&mut g);
        choose(&mut g, Action::GainCredit);
        encounter(&mut g, Server::Hq);
        // Three credits and one remaining click. Palisade costs one, but the
        // inner Brân cannot be crossed, nor can five-credit/two-click Manegarm.
        assert!(
            g.observe(Player::Runner)
                .decision
                .unwrap()
                .choices
                .iter()
                .any(|a| matches!(a, Action::Break { .. }))
        );
        assert_eq!(
            Policy::RunnerV1.choose(&g.observe(Player::Runner)),
            Some(Action::PassPriority)
        );
    }
}
#[test]
fn economy_and_draw_have_real_opportunity_costs() {
    for (credits, expected) in [(0, Action::GainCredit), (8, Action::Draw)] {
        let mut g = position(
            &[
                (CardKind::SureGamble, Zone::Stack),
                (CardKind::Cleaver, Zone::Stack),
                (CardKind::HedgeFund, Zone::Hq),
            ],
            credits,
        );
        action_window(&mut g);
        assert_eq!(
            Policy::RunnerV1.choose(&g.observe(Player::Runner)),
            Some(expected)
        );
    }
}
#[test]
fn an_unprotected_scoring_remote_is_contested_and_a_known_lethal_trap_is_not() {
    for (kind, should_run) in [
        (CardKind::OffworldOffice, true),
        (CardKind::UrticaCipher, false),
    ] {
        let mut p = EngineeringPosition::economy(3);
        p.active = Player::Runner;
        p.corp_credits = 0;
        p.runner_credits = 5;
        p.shuffle_decks = false;
        p.cards = vec![
            EngineeringCard {
                kind,
                zone: Zone::Root(Server::Remote(1)),
                advancement: 3,
                installed_turn: Some(1),
                rezzed: kind == CardKind::UrticaCipher,
            },
            EngineeringCard::new(CardKind::SureGamble, Zone::Grip),
        ];
        if should_run {
            p.cards
                .extend((0..4).map(|_| EngineeringCard::new(CardKind::Jailbreak, Zone::Grip)));
        }
        let mut g = Match::engineering_turns(p).unwrap();
        action_window(&mut g);
        let action = Policy::RunnerV1.choose(&g.observe(Player::Runner)).unwrap();
        assert_eq!(action == Action::RunServer(Server::Remote(1)), should_run);
    }
}
fn reach_encounter(g: &mut Match) {
    for _ in 0..40 {
        if g.observe(Player::Runner)
            .run
            .as_ref()
            .is_some_and(|r| r.encounter_paid)
        {
            return;
        }
        let v = g.observe(Player::Runner);
        choose(
            g,
            if v.decision
                .as_ref()
                .is_some_and(|d| d.choices.contains(&Action::Continue))
            {
                Action::Continue
            } else {
                Action::PassPriority
            },
        );
    }
    panic!("no encounter");
}
#[test]
fn temporary_payment_preserves_the_pool_for_whitespaces_conditional_end() {
    let mut g = position(
        &[
            (CardKind::Unity, Zone::Rig),
            (CardKind::Overclock, Zone::Grip),
            (CardKind::Whitespace, Zone::Protecting(Server::Hq)),
        ],
        8,
    );
    action_window(&mut g);
    let event = id(&g, "30029");
    let unity = id(&g, "30026");
    choose(&mut g, Action::PlayCard(event));
    choose(&mut g, Action::RunServer(Server::Hq));
    reach_encounter(&mut g);
    // Seven pool + five temporary. Break the pool loss for one temporary credit;
    // seven pool then makes the second subroutine harmless. Breaking the second
    // first instead loses three permanent credits when the first resolves.
    for expected in [
        Action::Break {
            breaker: unity,
            subroutines: 1,
        },
        Action::PayRunCredits(1),
        Action::PassPriority,
    ] {
        assert_eq!(
            Policy::RunnerV1.choose(&g.observe(Player::Runner)),
            Some(expected)
        );
        choose(&mut g, expected);
    }
    assert_eq!(g.observe(Player::Runner).runner_credits, 7);
    assert_eq!(g.observe(Player::Runner).run.unwrap().hosted_credits, 4);
}
#[test]
fn karuna_continuation_jacks_out_of_exposed_lethal_but_can_accept_nonlethal_damage() {
    for (hand, expected) in [(3, Action::JackOut), (4, Action::Continue)] {
        let mut cards = vec![(CardKind::Karuna, Zone::Protecting(Server::Hq))];
        cards.extend((0..hand).map(|_| (CardKind::SureGamble, Zone::Grip)));
        let mut g = position(&cards, 0);
        encounter(&mut g, Server::Hq);
        choose(&mut g, Action::PassPriority);
        choose(&mut g, Action::PassPriority);
        assert_eq!(g.observe(Player::Runner).hand.len(), hand - 2);
        assert!(
            g.observe(Player::Runner)
                .run
                .as_ref()
                .unwrap()
                .karuna_continuation
        );
        assert_eq!(
            Policy::RunnerV1.choose(&g.observe(Player::Runner)),
            Some(expected)
        );
    }
}
#[test]
fn mayfly_is_used_when_needed_and_its_delayed_loss_is_real() {
    let mut g = position(
        &[
            (CardKind::Mayfly, Zone::Rig),
            (CardKind::Palisade, Zone::Protecting(Server::Hq)),
            (CardKind::HedgeFund, Zone::Hq),
        ],
        2,
    );
    encounter(&mut g, Server::Hq);
    let mayfly = id(&g, "30032");
    for expected in [
        Action::Boost(mayfly),
        Action::PayRunCredits(0),
        Action::Break {
            breaker: mayfly,
            subroutines: 1,
        },
        Action::PayRunCredits(0),
    ] {
        assert_eq!(
            Policy::RunnerV1.choose(&g.observe(Player::Runner)),
            Some(expected)
        );
        choose(&mut g, expected);
    }
    assert_eq!(
        g.observe(Player::Runner).run.unwrap().trash_at_run_end,
        vec![mayfly]
    );
    for _ in 0..30 {
        if g.observe(Player::Runner).run.is_none() {
            break;
        }
        let actor = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|p| g.observe(*p).decision.is_some())
            .unwrap();
        let v = g.observe(actor);
        let a = if actor == Player::Runner {
            Policy::RunnerV1.choose(&v).unwrap()
        } else {
            Policy::CorpV1.choose(&v).unwrap()
        };
        choose(&mut g, a);
    }
    assert!(g.observe(Player::Runner).rig.is_empty());
    assert_eq!(g.observe(Player::Runner).heap[0].instance, Some(mayfly));
}
#[test]
fn secret_permutations_and_game_rng_do_not_change_the_policy_input_or_choice() {
    let a = EngineeringPosition::hq_access(3);
    let mut b = a.clone();
    b.seed = 42;
    for zone in [Zone::Hq, Zone::Rd, Zone::Stack] {
        let indices: Vec<_> = b
            .cards
            .iter()
            .enumerate()
            .filter(|(_, c)| c.zone == zone)
            .map(|(i, _)| i)
            .collect();
        if indices.len() > 1 {
            let first = indices[0];
            let last = *indices.last().unwrap();
            let kind = b.cards[first].kind;
            b.cards[first].kind = b.cards[last].kind;
            b.cards[last].kind = kind;
        }
    }
    let ga = Match::engineering_turns(a).unwrap();
    let gb = Match::engineering_turns(b).unwrap();
    let va = ga.observe(Player::Runner);
    let vb = gb.observe(Player::Runner);
    assert_eq!(va, vb);
    assert_eq!(Policy::RunnerV1.choose(&va), Policy::RunnerV1.choose(&vb));
}
#[test]
fn every_reached_admitted_choice_has_a_deliberate_legal_response() {
    for p in [
        EngineeringPosition::hq_access(3),
        EngineeringPosition::draw_economy(42),
        EngineeringPosition::breaker_run(89),
        EngineeringPosition::damage_run(117, true),
        EngineeringPosition::turn_income(3),
    ] {
        let mut g = Match::engineering_turns(p).unwrap();
        for _ in 0..200 {
            if g.observe(Player::Runner).match_winner.is_some() {
                break;
            }
            let actor = [Player::Runner, Player::Corp]
                .into_iter()
                .find(|p| g.observe(*p).decision.is_some())
                .unwrap();
            let view = g.observe(actor);
            let policy = if actor == Player::Runner {
                Policy::RunnerV1
            } else {
                Policy::CorpV1
            };
            let action = policy.choose(&view).unwrap_or_else(|| {
                panic!(
                    "no deliberate response to {} {:?}",
                    view.phase, view.decision
                )
            });
            choose(&mut g, action);
        }
    }
}
#[test]
fn central_pressure_moves_to_fresh_information_after_an_hq_breach() {
    let mut cards = vec![
        (CardKind::HedgeFund, Zone::Hq),
        (CardKind::HedgeFund, Zone::Rd),
    ];
    cards.extend((0..5).map(|_| (CardKind::SmartwareDistributor, Zone::Grip)));
    let mut g = position(&cards, 5);
    action_window(&mut g);
    assert_eq!(
        Policy::RunnerV1.choose(&g.observe(Player::Runner)),
        Some(Action::RunServer(Server::Hq))
    );
    choose(&mut g, Action::RunServer(Server::Hq));
    for _ in 0..35 {
        if g.observe(Player::Runner).run.is_none() {
            break;
        }
        let actor = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|p| g.observe(*p).decision.is_some())
            .unwrap();
        let policy = if actor == Player::Runner {
            Policy::RunnerV1
        } else {
            Policy::CorpV1
        };
        let a = policy.choose(&g.observe(actor)).unwrap();
        choose(&mut g, a);
    }
    action_window(&mut g);
    let view = g.observe(Player::Runner);
    assert!(view.successful_run_this_turn);
    assert!(view.hq_breached_this_turn);
    assert_eq!(view.central_runs, vec![Server::Hq]);
    assert_eq!(
        Policy::RunnerV1.choose(&view),
        Some(Action::RunServer(Server::Rd))
    );
    let resumed = Match::resume(g.checkpoint()).unwrap();
    assert_eq!(view, resumed.observe(Player::Runner));
    assert_eq!(
        Policy::RunnerV1.choose(&view),
        Policy::RunnerV1.choose(&resumed.observe(Player::Runner))
    );
}
#[test]
fn legitimately_revealed_ice_changes_the_line_without_revealing_the_hidden_twin() {
    let mut p = EngineeringPosition::economy(3);
    p.active = Player::Runner;
    p.corp_credits = 0;
    p.runner_credits = 1;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::Palisade, Zone::Protecting(Server::Hq)),
        EngineeringCard::new(CardKind::Bran, Zone::Rd),
        EngineeringCard::new(CardKind::Cleaver, Zone::Rig),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
    ];
    p.cards[0].installed_turn = Some(1);
    p.cards[2].installed_turn = Some(1);
    let mut q = p.clone();
    q.cards[0].kind = CardKind::Bran;
    q.cards[1].kind = CardKind::Palisade;
    let hidden_a = Match::engineering_turns(p.clone())
        .unwrap()
        .observe(Player::Runner);
    let hidden_b = Match::engineering_turns(q.clone())
        .unwrap()
        .observe(Player::Runner);
    assert_eq!(hidden_a, hidden_b);
    assert_eq!(
        Policy::RunnerV1.choose(&hidden_a),
        Policy::RunnerV1.choose(&hidden_b)
    );
    p.cards[0].rezzed = true;
    q.cards[0].rezzed = true;
    p.cards[0].installed_turn = Some(1);
    q.cards[0].installed_turn = Some(1);
    let mut a = Match::engineering_turns(p).unwrap();
    let mut b = Match::engineering_turns(q).unwrap();
    encounter(&mut a, Server::Hq);
    encounter(&mut b, Server::Hq);
    let va = a.observe(Player::Runner);
    let vb = b.observe(Player::Runner);
    assert_ne!(va, vb);
    assert!(matches!(
        Policy::RunnerV1.choose(&va),
        Some(Action::Break { .. })
    ));
    assert_ne!(Policy::RunnerV1.choose(&va), Policy::RunnerV1.choose(&vb));
}
#[test]
fn later_exposed_damage_prevents_throwing_away_the_first_break_payment() {
    let mut g = position(
        &[
            (CardKind::Cleaver, Zone::Rig),
            (CardKind::SureGamble, Zone::Grip),
            (CardKind::Karuna, Zone::Protecting(Server::Hq)),
            (CardKind::Palisade, Zone::Protecting(Server::Hq)),
        ],
        1,
    );
    encounter(&mut g, Server::Hq);
    assert_eq!(
        Policy::RunnerV1.choose(&g.observe(Player::Runner)),
        Some(Action::PassPriority)
    );
}
#[test]
fn impossible_later_ice_does_not_authorize_taking_lethal_damage_now() {
    let mut g = position(
        &[
            (CardKind::Carmen, Zone::Rig),
            (CardKind::Bran, Zone::Protecting(Server::Hq)),
            (CardKind::Tithe, Zone::Protecting(Server::Hq)),
        ],
        0,
    );
    action_window(&mut g);
    choose(&mut g, Action::GainCredit);
    action_window(&mut g);
    choose(&mut g, Action::GainCredit);
    encounter(&mut g, Server::Hq);
    // No grip: allowing Tithe's damage flatlines immediately. Spend one to
    // survive, then jack out at movement before the unaffordable inner Brân.
    assert_eq!(
        Policy::RunnerV1.choose(&g.observe(Player::Runner)),
        Some(Action::Break {
            breaker: id(&g, "30015"),
            subroutines: 1
        })
    );
}

#[test]
fn dense_introductory_board_returns_a_legal_choice_within_the_worker_budget() {
    // Exact starter inventory, all thirteen ICE publicly rezzed on HQ, every
    // breaker family plus Mayfly installed within Pennyshaver's five MU.
    let mut p = EngineeringPosition::hq_access(3);
    p.active = Player::Runner;
    p.runner_credits = 50;
    p.corp_credits = 0;
    p.shuffle_decks = false;
    for c in &mut p.cards {
        c.zone = if matches!(
            c.zone,
            Zone::Hq
                | Zone::Rd
                | Zone::Archives
                | Zone::Protecting(_)
                | Zone::Root(_)
                | Zone::CorpScore
                | Zone::ProtectingHq
        ) {
            Zone::Rd
        } else {
            Zone::Stack
        };
        c.rezzed = false;
        c.advancement = 0;
        c.installed_turn = None;
        if matches!(
            c.kind,
            CardKind::Bran
                | CardKind::Palisade
                | CardKind::Whitespace
                | CardKind::Tithe
                | CardKind::Diviner
                | CardKind::Karuna
        ) {
            c.zone = Zone::Protecting(Server::Hq);
            c.rezzed = true;
            c.installed_turn = Some(1);
        }
    }
    for kind in [
        CardKind::Cleaver,
        CardKind::Unity,
        CardKind::Carmen,
        CardKind::Mayfly,
        CardKind::Pennyshaver,
    ] {
        let c = p.cards.iter_mut().find(|c| c.kind == kind).unwrap();
        c.zone = Zone::Rig;
        c.installed_turn = Some(1);
    }
    for c in p.cards.iter_mut().filter(|c| c.zone == Zone::Stack).take(5) {
        c.zone = Zone::Grip;
    }
    let mut g = Match::engineering_turns(p).unwrap();
    action_window(&mut g);
    let view = g.observe(Player::Runner);
    assert_eq!(view.memory_used, 5);
    assert_eq!(view.servers[0].ice.len(), 13);
    let start = std::time::Instant::now();
    let action = Policy::RunnerV1.choose(&view).unwrap();
    let elapsed = start.elapsed();
    println!("dense introductory selector: {} ns", elapsed.as_nanos());
    assert!(view.decision.unwrap().choices.contains(&action));
    assert!(elapsed < std::time::Duration::from_secs(1));
}

#[test]
fn manegarm_is_refused_when_the_now_exposed_access_would_flatline() {
    let mut p = EngineeringPosition::economy(3);
    p.active = Player::Runner;
    p.runner_credits = 5;
    p.corp_credits = 0;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard {
            kind: CardKind::UrticaCipher,
            zone: Zone::Root(Server::Remote(1)),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: true,
        },
        EngineeringCard {
            kind: CardKind::ManegarmSkunkworks,
            zone: Zone::Root(Server::Remote(1)),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: true,
        },
        EngineeringCard::new(CardKind::SureGamble, Zone::Grip),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    action_window(&mut g);
    choose(&mut g, Action::RunServer(Server::Remote(1)));
    for _ in 0..25 {
        let view = g.observe(Player::Runner);
        if view
            .decision
            .as_ref()
            .is_some_and(|d| d.choices.contains(&Action::RefuseManegarm))
        {
            assert_eq!(Policy::RunnerV1.choose(&view), Some(Action::RefuseManegarm));
            return;
        }
        choose(
            &mut g,
            if view
                .decision
                .as_ref()
                .is_some_and(|d| d.choices.contains(&Action::Continue))
            {
                Action::Continue
            } else {
                Action::PassPriority
            },
        );
    }
    panic!("no Manegarm decision");
}
