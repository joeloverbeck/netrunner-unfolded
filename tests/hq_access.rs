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
fn pass(g: &mut Match) {
    choose(g, Action::PassPriority);
}
fn installed(kind: CardKind, zone: Zone, rezzed: bool) -> EngineeringCard {
    EngineeringCard {
        kind,
        zone,
        rezzed,
        installed_turn: Some(1),
        advancement: 0,
    }
}
fn approach() -> Match {
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.runner_credits = 4;
    p.shuffle_decks = false;
    let manegarm = serde_json::from_str("\"ManegarmSkunkworks\"").unwrap();
    p.cards = vec![
        installed(manegarm, Zone::Root(Server::Hq), true),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    for _ in 0..4 {
        pass(&mut g);
    }
    choose(&mut g, Action::RunServer(Server::Hq));
    for _ in 0..4 {
        pass(&mut g);
    }
    choose(&mut g, Action::Continue);
    for _ in 0..2 {
        pass(&mut g);
    }
    g
}
#[test]
fn manegarm_requires_two_clicks_or_five_credits_and_refusal_ends_run() {
    let g = approach();
    let view = g.observe(Player::Runner);
    assert_eq!(
        view.phase,
        "Manegarm · pay 5 credits or spend 2 clicks, otherwise end the run"
    );
    assert_eq!(view.clicks[1], 3);
    assert_eq!(view.runner_credits, 4);
}
#[test]
fn root_trash_is_optional_and_hq_steal_is_mandatory() {
    let mut g = approach();
    choose(&mut g, Action::SpendManegarmClicks);
    choose(&mut g, Action::AccessRoot(0));
    let v = g.observe(Player::Runner);
    assert_eq!(v.accessed.unwrap().printing, Some("30042"));
    assert!(v.decision.unwrap().choices.contains(&Action::FinishAccess));
    choose(&mut g, Action::TrashAccessed);
    choose(&mut g, Action::PayRunCredits(0));
    choose(&mut g, Action::AccessHq);
    assert_eq!(
        g.observe(Player::Runner).accessed.unwrap().printing,
        Some("30075")
    );
    choose(&mut g, Action::FinishAccess);
    assert!(g.observe(Player::Runner).run.is_none());
    assert_eq!(g.observe(Player::Runner).runner_credits, 1);
}
#[test]
fn hostile_case_contains_exact_introductory_multiplicities_and_inactive_remainder() {
    let p = EngineeringPosition::hq_access(42);
    let mut counts = std::collections::BTreeMap::new();
    for card in &p.cards {
        *counts.entry(card.kind.description().1).or_insert(0) += 1;
    }
    // Independently transcribed from #7's official introductory lists.
    let expected = [
        ("30012", 2),
        ("30020", 2),
        ("30021", 2),
        ("30029", 2),
        ("30028", 3),
        ("30030", 3),
        ("30013", 1),
        ("30014", 1),
        ("30006", 2),
        ("30015", 2),
        ("30026", 2),
        ("30032", 2),
        ("30018", 1),
        ("30027", 2),
        ("30033", 2),
        ("30034", 1),
        ("30067", 3),
        ("30069", 2),
        ("30070", 2),
        ("30037", 2),
        ("30045", 2),
        ("30071", 2),
        ("30039", 2),
        ("30046", 2),
        ("30047", 2),
        ("30072", 3),
        ("30073", 2),
        ("30074", 2),
        ("30040", 2),
        ("30064", 2),
        ("30075", 3),
        ("30042", 1),
    ]
    .into_iter()
    .collect();
    assert_eq!(counts, expected);
    assert_eq!(p.cards.len(), 64);
    assert_eq!(
        p.cards
            .iter()
            .filter(|c| matches!(c.zone, Zone::Rig | Zone::Grip | Zone::Stack | Zone::Heap))
            .count(),
        30
    );
    assert!(Match::engineering_turns(p).is_ok());
}
fn id(g: &Match, player: Player, title: &str) -> netrunner_unfolded::game::CardId {
    let v = g.observe(player);
    v.hand
        .iter()
        .chain(&v.rig)
        .chain(&v.archives)
        .chain(v.servers.iter().flat_map(|s| s.root.iter().chain(&s.ice)))
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}
fn until(g: &mut Match, phase: &str) {
    for _ in 0..24 {
        if g.observe(Player::Runner).phase.starts_with(phase) {
            return;
        }
        pass(g);
    }
    panic!("did not reach {phase}");
}
fn start_hostile() -> Match {
    start_hostile_position(EngineeringPosition::hq_access(42))
}
fn start_hostile_position(p: EngineeringPosition) -> Match {
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    let event = id(&g, Player::Runner, "Overclock");
    choose(&mut g, Action::PlayCard(event));
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Encounter");
    pass(&mut g);
    pass(&mut g);
    let palisade = id(&g, Player::Corp, "Palisade");
    let bran = id(&g, Player::Corp, "Brân 1.0");
    choose(&mut g, Action::InstallInward(palisade));
    choose(&mut g, Action::TrashInstalled(bran));
    choose(&mut g, Action::FinishInstall);
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Approach ICE");
    pass(&mut g);
    choose(&mut g, Action::Rez(palisade));
    pass(&mut g);
    pass(&mut g);
    let cleaver = id(&g, Player::Runner, "Cleaver");
    choose(
        &mut g,
        Action::Break {
            breaker: cleaver,
            subroutines: 1,
        },
    );
    choose(&mut g, Action::PayRunCredits(1));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Encounter");
    let unity = id(&g, Player::Runner, "Unity");
    choose(
        &mut g,
        Action::Break {
            breaker: unity,
            subroutines: 2,
        },
    );
    choose(&mut g, Action::PayRunCredits(1));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Manegarm");
    g
}
#[test]
fn all_nine_hostile_steps_preserve_scores_resources_and_the_next_run() {
    let mut g = start_hostile();
    let v = g.observe(Player::Runner);
    assert_eq!(
        (
            v.runner_credits,
            v.corp_credits,
            v.clicks[1],
            v.run.unwrap().hosted_credits
        ),
        (4, 0, 3, 3)
    );
    assert_eq!(
        v.decision.unwrap().choices,
        vec![
            Action::PayManegarmCredits,
            Action::SpendManegarmClicks,
            Action::RefuseManegarm
        ]
    );
    choose(&mut g, Action::SpendManegarmClicks);
    assert_eq!(
        g.observe(Player::Runner).run.unwrap().hq_access_remaining,
        Some(2)
    );
    choose(&mut g, Action::AccessRoot(0));
    choose(&mut g, Action::TrashAccessed);
    choose(&mut g, Action::PayRunCredits(3));
    for _ in 0..2 {
        choose(&mut g, Action::AccessHq);
        let v = g.observe(Player::Runner);
        if v.accessed.unwrap().printing == Some("30067") {
            assert_eq!(v.decision.unwrap().choices, vec![Action::StealAgenda]);
            choose(&mut g, Action::StealAgenda);
        } else {
            choose(&mut g, Action::FinishAccess);
        }
    }
    let v = g.observe(Player::Runner);
    assert!(v.run.is_none());
    assert_eq!(v.agenda_points, [0, 2]);
    assert_eq!(v.corp_credits, 0);
    until(&mut g, "Action phase");
    choose(&mut g, Action::GainCredit);
    until(&mut g, "Action phase");
    assert_eq!(g.observe(Player::Corp).active_player, Some(Player::Corp));
    choose(&mut g, Action::GainCredit);
    until(&mut g, "Action phase");
    let seamless = id(&g, Player::Corp, "Seamless Launch");
    choose(&mut g, Action::PlayCard(seamless));
    let office = id(&g, Player::Corp, "Offworld Office");
    choose(&mut g, Action::SelectTarget(office));
    assert_eq!(
        g.observe(Player::Corp)
            .servers
            .iter()
            .find(|s| s.server == Server::Remote(1))
            .unwrap()
            .root[0]
            .advancement,
        4
    );
    choose(&mut g, Action::Score(office));
    until(&mut g, "Action phase");
    choose(&mut g, Action::GainCredit);
    until(&mut g, "Action phase");
    let v = g.observe(Player::Runner);
    assert_eq!(
        (v.runner_credits, v.corp_credits, v.agenda_points),
        (5, 8, [2, 2])
    );
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Encounter");
    let cleaver = id(&g, Player::Runner, "Cleaver");
    choose(
        &mut g,
        Action::Break {
            breaker: cleaver,
            subroutines: 1,
        },
    );
    choose(&mut g, Action::PayRunCredits(0));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Encounter");
    let unity = id(&g, Player::Runner, "Unity");
    for mask in [1, 2] {
        choose(
            &mut g,
            Action::Break {
                breaker: unity,
                subroutines: mask,
            },
        );
        choose(&mut g, Action::PayRunCredits(0));
    }
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Breach HQ");
    let v = g.observe(Player::Runner);
    assert_eq!(v.runner_credits, 2);
    assert!(v.servers[0].root.is_empty());
    assert_eq!(v.servers[0].ice[1].printing, Some("30072"));
    assert!(v.archives.iter().any(|c| c.printing == Some("30039")));
}

#[test]
fn manegarm_payment_splits_refusal_and_clicks_remain_distinct() {
    for hosted in [1, 2, 3] {
        let mut g = start_hostile();
        choose(&mut g, Action::PayManegarmCredits);
        assert_eq!(
            g.observe(Player::Runner).decision.unwrap().choices,
            vec![
                Action::PayRunCredits(1),
                Action::PayRunCredits(2),
                Action::PayRunCredits(3)
            ]
        );
        choose(&mut g, Action::PayRunCredits(hosted));
        let v = g.observe(Player::Runner);
        assert_eq!(
            (v.clicks[1], v.runner_credits, v.run.unwrap().hosted_credits),
            (3, u32::from(hosted - 1), 3 - hosted)
        );
    }
    let mut g = start_hostile();
    choose(&mut g, Action::RefuseManegarm);
    let v = g.observe(Player::Runner);
    assert_eq!((v.runner_credits, v.clicks[1]), (4, 3));
    assert_eq!(
        v.run_result,
        Some(netrunner_unfolded::game::RunResult::EndedByManegarm)
    );
    assert!(v.run.is_none());
    assert!(v.heap.iter().any(|c| c.printing == Some("30029")));
}
#[test]
fn hq_first_order_and_remembered_faces_survive_restart_without_hidden_tracking_ids() {
    let mut g = start_hostile();
    choose(&mut g, Action::SpendManegarmClicks);
    choose(&mut g, Action::AccessHq);
    let v = g.observe(Player::Runner);
    let accessed = v.accessed.unwrap();
    assert!(accessed.instance.is_none());
    assert_eq!(accessed.printing, Some("30075")); // shared SplitMix64 seed 42, two HQ candidates
    choose(&mut g, Action::FinishAccess);
    assert_eq!(
        g.observe(Player::Runner).run.unwrap().known_hq[0].printing,
        Some("30075")
    );
    choose(&mut g, Action::AccessHq);
    assert_eq!(
        g.observe(Player::Runner).accessed.unwrap().printing,
        Some("30067")
    );
    choose(&mut g, Action::StealAgenda);
    assert_eq!(
        g.observe(Player::Runner).decision.unwrap().choices,
        vec![Action::AccessRoot(0)]
    );
    choose(&mut g, Action::AccessRoot(0));
    choose(&mut g, Action::FinishAccess);
    assert!(g.observe(Player::Runner).run.is_none());
    assert!(
        g.observe(Player::Runner)
            .history
            .iter()
            .any(|s| s == "Runner accesses Hedge Fund.")
    );
    assert_eq!(g.observe(Player::Runner).servers[0].root.len(), 1);
}
#[test]
fn paired_hidden_decks_and_hq_do_not_change_inputs_policy_or_manegarm_restart() {
    let first = EngineeringPosition::hq_access(42);
    let mut second = first.clone();
    for c in &mut second.cards {
        if c.zone == Zone::Hq {
            c.kind = match c.kind {
                CardKind::OffworldOffice => CardKind::HedgeFund,
                CardKind::HedgeFund => CardKind::OffworldOffice,
                k => k,
            };
        }
    }
    let a = start_hostile_position(first.clone());
    let b = start_hostile_position(second);
    assert_ne!(a.observe(Player::Corp), b.observe(Player::Corp));
    assert_eq!(a.observe(Player::Runner), b.observe(Player::Runner));
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&a.observe(Player::Runner)),
        netrunner_unfolded::game::baseline_response(&b.observe(Player::Runner))
    );
    for g in [a, b] {
        let resumed = Match::resume(g.checkpoint()).unwrap();
        assert_eq!(g.observe(Player::Runner), resumed.observe(Player::Runner));
    }
    let a = start_hostile_position(first.clone());
    let mut second = first;
    let indices: Vec<_> = second
        .cards
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (c.zone == Zone::Stack).then_some(i))
        .collect();
    second.cards.swap(indices[0], *indices.last().unwrap());
    let b = start_hostile_position(second);
    for p in [Player::Corp, Player::Runner] {
        assert_eq!(a.observe(p), b.observe(p));
        assert_eq!(
            netrunner_unfolded::game::baseline_response(&a.observe(p)),
            netrunner_unfolded::game::baseline_response(&b.observe(p))
        );
    }
}
#[test]
fn unsupported_live_remainder_is_refused_atomically_instead_of_becoming_an_inert_card() {
    use netrunner_unfolded::play::{Play, SaveOptions};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inactive.json");
    let mut play = Play::engineering(
        EngineeringPosition::hq_access(42),
        Some(SaveOptions {
            path: path.clone(),
            recover_from: None,
        }),
        Player::Runner,
    );
    play.admit();
    for _ in 0..8 {
        let v = play.observation().unwrap();
        if v.phase == "Action phase" {
            break;
        }
        if let Some(d) = v.decision {
            play.choose(d.revision, Action::PassPriority).unwrap();
        } else {
            play.respond().unwrap();
        }
    }
    let before = std::fs::read(&path).unwrap();
    let v = play.observation().unwrap();
    let error = play
        .choose(v.decision.as_ref().unwrap().revision, Action::Draw)
        .unwrap_err();
    assert!(!error.contains("Tread") && !error.contains("30012"));
    assert_eq!(v, play.observation().unwrap());
    assert_eq!(before, std::fs::read(&path).unwrap());
}
#[test]
fn docklands_counts_actual_breaches_and_resets_only_at_turn_boundaries() {
    let mut p = EngineeringPosition::economy(7);
    p.active = Player::Runner;
    p.shuffle_decks = false;
    p.cards = vec![
        installed(CardKind::DocklandsPass, Zone::Rig, false),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
        EngineeringCard::new(CardKind::Palisade, Zone::Hq),
        EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Hq),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    for expected in [2, 1] {
        choose(&mut g, Action::RunServer(Server::Hq));
        until(&mut g, "Movement");
        choose(&mut g, Action::Continue);
        until(&mut g, "Breach HQ");
        assert_eq!(
            g.observe(Player::Runner).run.unwrap().hq_access_remaining,
            Some(expected)
        );
        for _ in 0..expected {
            choose(&mut g, Action::AccessHq);
            choose(&mut g, Action::FinishAccess);
        }
        until(&mut g, "Action phase");
    }
    for _ in 0..5 {
        choose(&mut g, Action::GainCredit);
        until(&mut g, "Action phase");
    }
    assert_eq!(g.observe(Player::Runner).turn_number, 4);
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Breach HQ");
    assert_eq!(
        g.observe(Player::Runner).run.unwrap().hq_access_remaining,
        Some(2)
    );
}
#[test]
fn manegarm_can_rez_in_the_last_window_after_the_last_jack_out_choice() {
    let mut p = EngineeringPosition::economy(7);
    p.active = Player::Runner;
    p.corp_credits = 2;
    p.shuffle_decks = false;
    p.cards = vec![installed(
        CardKind::ManegarmSkunkworks,
        Zone::Root(Server::Hq),
        false,
    )];
    let mut g = Match::engineering_turns(p).unwrap();
    // Decline the earlier ordinary paid-window rez opportunities.
    until(&mut g, "Action phase");
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    pass(&mut g);
    let card = id(&g, Player::Corp, "Manegarm Skunkworks");
    choose(&mut g, Action::Rez(card));
    pass(&mut g);
    pass(&mut g);
    assert!(g.observe(Player::Runner).phase.starts_with("Manegarm"));
    assert_eq!(g.observe(Player::Runner).corp_credits, 0);
}

#[test]
fn remembered_hq_faces_follow_lawful_access_history_not_hidden_hand_order() {
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::Palisade, Zone::Hq),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
        installed(CardKind::DocklandsPass, Zone::Rig, false),
        installed(CardKind::ManegarmSkunkworks, Zone::Root(Server::Hq), true),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Manegarm");
    choose(&mut g, Action::SpendManegarmClicks);
    for printing in ["30075", "30072"] {
        choose(&mut g, Action::AccessHq);
        assert_eq!(
            g.observe(Player::Runner).accessed.unwrap().printing,
            Some(printing)
        );
        choose(&mut g, Action::FinishAccess);
    }
    let known = g.observe(Player::Runner).run.unwrap().known_hq;
    assert_eq!(
        known.iter().map(|c| c.printing).collect::<Vec<_>>(),
        vec![Some("30075"), Some("30072")]
    );
    assert!(known.iter().all(|c| c.instance.is_none()));
}
