//! CR 26.03 10.4 / 1.7.2b and the pinned Gateway card clauses.
use netrunner_unfolded::game::{
    Action, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
};
fn choose(g: &mut Match, action: Action) {
    let d = [Player::Runner, Player::Corp]
        .into_iter()
        .find_map(|p| g.observe(p).decision)
        .unwrap();
    assert!(d.choices.contains(&action), "{action:?}: {:?}", d.choices);
    g.choose(d.actor, d.revision, action).unwrap();
    let resumed = Match::resume(g.checkpoint()).unwrap();
    for p in [Player::Runner, Player::Corp] {
        assert_eq!(g.observe(p), resumed.observe(p));
    }
}
fn until(g: &mut Match, phase: &str) {
    for _ in 0..30 {
        if g.observe(Player::Runner).phase.starts_with(phase) {
            return;
        }
        choose(g, Action::PassPriority);
    }
    panic!("did not reach {phase}");
}
fn encounter(kind: CardKind, grip: &[CardKind]) -> Match {
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.shuffle_decks = false;
    p.cards = grip
        .iter()
        .map(|&k| EngineeringCard::new(k, Zone::Grip))
        .collect();
    p.cards.push(EngineeringCard {
        kind,
        zone: Zone::Protecting(Server::Hq),
        rezzed: true,
        installed_turn: Some(1),
        advancement: 0,
    });
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Encounter");
    g
}
#[test]
fn tithe_damage_and_credit_are_distinct_and_flatline_stops_credit() {
    let mut g = encounter(CardKind::Tithe, &[CardKind::SureGamble]);
    let credits = g.observe(Player::Corp).corp_credits;
    choose(&mut g, Action::PassPriority);
    choose(&mut g, Action::PassPriority);
    let v = g.observe(Player::Runner);
    assert_eq!(v.hand.len(), 0);
    assert_eq!(v.heap.len(), 1);
    assert_eq!(v.match_winner, None);
    assert_eq!(v.corp_credits, credits + 1);
    let mut lethal = encounter(CardKind::Tithe, &[]);
    choose(&mut lethal, Action::PassPriority);
    choose(&mut lethal, Action::PassPriority);
    let v = lethal.observe(Player::Runner);
    assert_eq!(v.match_winner, Some(Player::Corp));
    assert_eq!(v.corp_credits, credits);
    assert!(v.decision.is_none());
}
#[test]
fn diviner_tests_the_actual_randomly_trashed_cards_printed_cost() {
    for (card, ends) in [
        (CardKind::SureGamble, true),
        (CardKind::Cleaver, true),
        (CardKind::DocklandsPass, false),
        (CardKind::Jailbreak, false),
    ] {
        let mut g = encounter(CardKind::Diviner, &[card]);
        choose(&mut g, Action::PassPriority);
        choose(&mut g, Action::PassPriority);
        let v = g.observe(Player::Runner);
        assert_eq!(v.heap[0].printing, Some(card.description().1));
        assert_eq!(v.run.is_none(), ends);
        assert_eq!(v.match_winner, None);
    }
}
#[test]
fn karuna_suspends_only_for_jack_out_and_continues_remaining_damage() {
    let mut g = encounter(CardKind::Karuna, &[CardKind::Jailbreak; 4]);
    choose(&mut g, Action::PassPriority);
    choose(&mut g, Action::PassPriority);
    let v = g.observe(Player::Runner);
    assert_eq!((v.hand.len(), v.heap.len()), (2, 2));
    assert_eq!(
        v.decision.unwrap().choices,
        vec![Action::Continue, Action::JackOut]
    );
    let mut jack = Match::resume(g.checkpoint()).unwrap();
    choose(&mut jack, Action::JackOut);
    assert!(jack.observe(Player::Runner).run.is_none());
    assert_eq!(jack.observe(Player::Runner).hand.len(), 2);
    choose(&mut g, Action::Continue);
    let v = g.observe(Player::Runner);
    assert_eq!((v.hand.len(), v.heap.len(), v.match_winner), (0, 4, None));
    let mut lethal = encounter(CardKind::Karuna, &[CardKind::SureGamble]);
    choose(&mut lethal, Action::PassPriority);
    choose(&mut lethal, Action::PassPriority);
    assert_eq!(
        lethal.observe(Player::Runner).match_winner,
        Some(Player::Corp)
    );
    assert!(lethal.observe(Player::Runner).decision.is_none());
}
fn urtica(zone: Zone, advancement: u32, hand: usize) -> Match {
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.shuffle_decks = false;
    p.cards = (0..hand)
        .map(|_| EngineeringCard::new(CardKind::Jailbreak, Zone::Grip))
        .collect();
    p.cards.push(EngineeringCard {
        kind: CardKind::UrticaCipher,
        zone,
        advancement,
        installed_turn: matches!(zone, Zone::Root(_)).then_some(1),
        rezzed: false,
    });
    let server = match zone {
        Zone::Root(s) => s,
        Zone::Hq => Server::Hq,
        Zone::Rd => Server::Rd,
        _ => Server::Archives,
    };
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    choose(&mut g, Action::RunServer(server));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Breach");
    g
}
#[test]
fn installed_urtica_deals_one_occurrence_before_trash_and_zero_counters_still_deal_two() {
    for counters in [0, 2] {
        let mut g = urtica(Zone::Root(Server::Remote(1)), counters, 5);
        let before = g.observe(Player::Runner);
        assert_eq!(before.servers.last().unwrap().root[0].title, None);
        choose(&mut g, Action::AccessRoot(0));
        let v = g.observe(Player::Runner);
        assert_eq!(v.hand.len(), 3 - counters as usize);
        assert_eq!(v.heap.len(), 2 + counters as usize);
        assert_eq!(
            v.history
                .iter()
                .filter(|h| h.contains("net damage;"))
                .count(),
            1
        );
        assert_eq!(v.accessed.unwrap().printing, Some("30045"));
        choose(&mut g, Action::TrashAccessed);
        assert_eq!(g.observe(Player::Runner).run.unwrap().payment_cost, Some(2));
        choose(&mut g, Action::PayRunCredits(0));
        let v = g.observe(Player::Runner);
        assert_eq!(v.runner_credits, 3);
        assert_eq!(v.archives[0].printing, Some("30045"));
        assert!(v.run.is_none());
    }
    let mut lethal = urtica(Zone::Root(Server::Remote(1)), 2, 3);
    choose(&mut lethal, Action::AccessRoot(0));
    assert_eq!(
        lethal.observe(Player::Runner).match_winner,
        Some(Player::Corp)
    );
    assert!(lethal.observe(Player::Runner).decision.is_none());
}
#[test]
fn central_urtica_never_triggers_and_archives_has_no_trash_cost() {
    for (zone, access) in [
        (Zone::Hq, Action::AccessHq),
        (Zone::Rd, Action::AccessRd),
        (Zone::Archives, Action::AccessArchives(0)),
    ] {
        let mut g = urtica(zone, 0, 1);
        choose(&mut g, access);
        let v = g.observe(Player::Runner);
        assert_eq!((v.hand.len(), v.heap.len(), v.match_winner), (1, 0, None));
        if zone == Zone::Archives {
            assert_eq!(v.decision.unwrap().choices, vec![Action::FinishAccess]);
        }
        choose(&mut g, Action::FinishAccess);
    }
}
#[test]
fn random_damage_restarts_without_reroll_and_condition_follows_the_selected_card() {
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..16 {
        let mut p = EngineeringPosition::economy(seed);
        p.active = Player::Runner;
        p.shuffle_decks = false;
        p.cards = vec![
            EngineeringCard::new(CardKind::SureGamble, Zone::Grip),
            EngineeringCard::new(CardKind::Jailbreak, Zone::Grip),
            EngineeringCard {
                kind: CardKind::Diviner,
                zone: Zone::Protecting(Server::Hq),
                rezzed: true,
                advancement: 0,
                installed_turn: Some(1),
            },
        ];
        let mut g = Match::engineering_turns(p).unwrap();
        until(&mut g, "Action phase");
        choose(&mut g, Action::RunServer(Server::Hq));
        until(&mut g, "Encounter");
        let mut resumed = Match::resume(g.checkpoint()).unwrap();
        for game in [&mut g, &mut resumed] {
            choose(game, Action::PassPriority);
            choose(game, Action::PassPriority);
        }
        for player in [Player::Runner, Player::Corp] {
            assert_eq!(g.observe(player), resumed.observe(player));
        }
        let v = g.observe(Player::Runner);
        let card = v.heap[0].printing.unwrap();
        seen.insert(card);
        assert_eq!(v.run.is_none(), card == "30030");
        assert_eq!(v.hand.len(), 1);
        let corp = g.observe(Player::Corp);
        assert_eq!(corp.grip_count, 1);
        assert!(corp.hand.is_empty());
        assert_eq!(corp.heap[0].printing, Some(card));
    }
    assert_eq!(seen, ["30028", "30030"].into_iter().collect());
}
#[test]
fn lethal_damage_is_terminal_and_further_choices_are_atomic_refusals() {
    let mut g = encounter(CardKind::Karuna, &[CardKind::Jailbreak; 3]);
    choose(&mut g, Action::PassPriority);
    choose(&mut g, Action::PassPriority);
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&g.observe(Player::Runner)),
        Some(Action::JackOut)
    );
    choose(&mut g, Action::Continue);
    let v = g.observe(Player::Runner);
    assert_eq!(v.match_winner, Some(Player::Corp));
    assert_eq!(v.hand.len(), 1);
    let before = serde_json::to_value(g.checkpoint()).unwrap();
    assert!(g.choose(Player::Runner, 0, Action::Continue).is_err());
    assert_eq!(before, serde_json::to_value(g.checkpoint()).unwrap());
}
#[test]
fn rd_trash_payment_does_not_tell_corp_the_unseen_card_or_cost() {
    let mut trap = urtica(Zone::Rd, 0, 1);
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::Jailbreak, Zone::Grip),
        EngineeringCard::new(CardKind::ManegarmSkunkworks, Zone::Rd),
    ];
    let mut other = Match::engineering_turns(p).unwrap();
    until(&mut other, "Action phase");
    choose(&mut other, Action::RunServer(Server::Rd));
    until(&mut other, "Movement");
    choose(&mut other, Action::Continue);
    until(&mut other, "Breach");
    for g in [&mut trap, &mut other] {
        choose(g, Action::AccessRd);
        choose(g, Action::TrashAccessed);
    }
    assert_eq!(trap.observe(Player::Corp), other.observe(Player::Corp));
    assert_ne!(trap.observe(Player::Runner), other.observe(Player::Runner));
}
#[test]
fn overclock_tithe_diviner_and_installed_trap_share_one_run_and_payment() {
    let p = EngineeringPosition::damage_run(42, false);
    assert_eq!(p.cards.len(), 64);
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    let event = g
        .observe(Player::Runner)
        .hand
        .iter()
        .find(|c| c.printing == Some("30029"))
        .unwrap()
        .instance
        .unwrap();
    choose(&mut g, Action::PlayCard(event));
    choose(&mut g, Action::RunServer(Server::Remote(1)));
    for expected_hand in [3, 2] {
        until(&mut g, "Encounter");
        choose(&mut g, Action::PassPriority);
        choose(&mut g, Action::PassPriority);
        assert_eq!(g.observe(Player::Runner).hand.len(), expected_hand);
        until(&mut g, "Movement");
        choose(&mut g, Action::Continue);
    }
    until(&mut g, "Breach");
    choose(&mut g, Action::AccessRoot(0));
    let v = g.observe(Player::Runner);
    assert_eq!(
        (v.hand.len(), v.heap.len(), v.runner_credits, v.corp_credits),
        (0, 4, 7, 1)
    );
    assert_eq!(v.match_winner, None);
    choose(&mut g, Action::TrashAccessed);
    choose(&mut g, Action::PayRunCredits(2));
    let v = g.observe(Player::Runner);
    assert_eq!((v.heap.len(), v.runner_credits, v.corp_credits), (5, 7, 1));
    assert!(v.run.is_none());
    assert_eq!(v.clicks[1], 3);
}
#[test]
fn unrezzed_trap_stays_concealed_while_advancing_and_installation_respects_one_asset_or_agenda() {
    let mut p = EngineeringPosition::economy(42);
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::UrticaCipher, Zone::Hq),
        EngineeringCard {
            kind: CardKind::OffworldOffice,
            zone: Zone::Root(Server::Remote(1)),
            rezzed: false,
            installed_turn: Some(1),
            advancement: 0,
        },
        EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    let id = g
        .observe(Player::Corp)
        .hand
        .iter()
        .find(|c| c.printing == Some("30045"))
        .unwrap()
        .instance
        .unwrap();
    choose(&mut g, Action::InstallCard(id));
    let d = g.observe(Player::Corp).decision.unwrap();
    assert!(!d.choices.contains(&Action::Destination(Server::Hq)));
    choose(&mut g, Action::Destination(Server::Remote(1)));
    let v = g.observe(Player::Corp);
    assert!(
        !v.decision
            .as_ref()
            .unwrap()
            .choices
            .contains(&Action::FinishInstall)
    );
    let agenda = v.servers.last().unwrap().root[0].instance.unwrap();
    choose(&mut g, Action::TrashInstalled(agenda));
    choose(&mut g, Action::FinishInstall);
    until(&mut g, "Action phase");
    choose(&mut g, Action::Advance(id));
    let runner = g.observe(Player::Runner);
    let root = &runner.servers.last().unwrap().root[0];
    assert_eq!(root.title, None);
    assert_eq!(root.advancement, 1);
    until(&mut g, "Action phase");
    assert!(
        !g.observe(Player::Corp)
            .decision
            .unwrap()
            .choices
            .contains(&Action::Score(id))
    );
}
#[test]
fn corp_baseline_does_not_reveal_an_unrezzed_trap_without_a_benefit() {
    let mut g = Match::engineering_turns(EngineeringPosition::damage_run(42, true)).unwrap();
    until(&mut g, "Action phase");
    choose(&mut g, Action::RunServer(Server::Remote(1)));
    until(&mut g, "Approach ICE");
    choose(&mut g, Action::PassPriority);
    let v = g.observe(Player::Corp);
    assert!(
        v.decision
            .as_ref()
            .unwrap()
            .choices
            .iter()
            .any(|a| matches!(a, Action::Rez(_)))
    );
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&v),
        Some(Action::PassPriority)
    );
}
#[test]
fn saved_karuna_and_flatline_reject_dangling_sources_or_invented_damage() {
    let mut g = encounter(CardKind::Karuna, &[CardKind::Jailbreak; 3]);
    choose(&mut g, Action::PassPriority);
    choose(&mut g, Action::PassPriority);
    let mut pending = serde_json::to_value(g.checkpoint()).unwrap();
    pending["state"]["turn"]["run"]["stage"]["KarunaJackOut"]["source"] = serde_json::json!(99999);
    assert!(Match::resume(serde_json::from_value(pending).unwrap()).is_err());
    choose(&mut g, Action::Continue);
    let mut terminal = serde_json::to_value(g.checkpoint()).unwrap();
    terminal["state"]["turn"]["terminal"]["Flatline"]["damage"] = serde_json::json!(17);
    assert!(Match::resume(serde_json::from_value(terminal).unwrap()).is_err());
}
#[test]
fn admitted_saved_encounter_can_resolve_only_karunas_second_subroutine() {
    let g = encounter(CardKind::Karuna, &[CardKind::Jailbreak]);
    // A supported saved encounter can already contain a broken first subroutine;
    // this does not admit a sentry breaker or add an ordinary-play shortcut.
    let mut checkpoint = serde_json::to_value(g.checkpoint()).unwrap();
    checkpoint["state"]["turn"]["run"]["broken"] = serde_json::json!(1);
    let mut g = Match::resume(serde_json::from_value(checkpoint).unwrap()).unwrap();
    choose(&mut g, Action::PassPriority);
    choose(&mut g, Action::PassPriority);
    assert_eq!(g.observe(Player::Runner).match_winner, Some(Player::Corp));
    assert!(g.observe(Player::Runner).decision.is_none());
}
#[test]
fn traps_and_agendas_are_indistinguishable_to_runner_before_access() {
    let p = EngineeringPosition::damage_run(42, true);
    let mut q = p.clone();
    let trap = q
        .cards
        .iter()
        .position(|c| c.kind == CardKind::UrticaCipher && matches!(c.zone, Zone::Root(_)))
        .unwrap();
    let agenda = q
        .cards
        .iter()
        .position(|c| c.kind == CardKind::OffworldOffice)
        .unwrap();
    q.cards[trap].kind = CardKind::OffworldOffice;
    q.cards[agenda].kind = CardKind::UrticaCipher;
    let a = Match::engineering_turns(p).unwrap();
    let b = Match::engineering_turns(q).unwrap();
    assert_eq!(a.observe(Player::Runner), b.observe(Player::Runner));
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&a.observe(Player::Runner)),
        netrunner_unfolded::game::baseline_response(&b.observe(Player::Runner))
    );
    assert_ne!(a.observe(Player::Corp), b.observe(Player::Corp));
}
