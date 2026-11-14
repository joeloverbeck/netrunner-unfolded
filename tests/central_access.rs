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
    for _ in 0..24 {
        if g.observe(Player::Runner).phase.starts_with(phase) {
            return;
        }
        choose(g, Action::PassPriority);
    }
    panic!("did not reach {phase}");
}
fn installed(kind: CardKind, zone: Zone) -> EngineeringCard {
    EngineeringCard {
        kind,
        zone,
        rezzed: false,
        advancement: 0,
        installed_turn: Some(1),
    }
}
fn winning_position(points: u8) -> EngineeringPosition {
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.corp_credits = 0;
    p.agenda_points = [0, points];
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::Jailbreak, Zone::Grip),
        EngineeringCard::new(CardKind::SureGamble, Zone::Stack),
        EngineeringCard::new(CardKind::SendAMessage, Zone::Rd),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
        installed(CardKind::Palisade, Zone::Protecting(Server::Hq)),
    ];
    p
}
fn jailbreak(g: &mut Match) {
    until(g, "Action phase");
    let event = g
        .observe(Player::Runner)
        .hand
        .iter()
        .find(|c| c.printing == Some("30028"))
        .unwrap()
        .instance
        .unwrap();
    choose(g, Action::PlayCard(event));
    choose(g, Action::RunServer(Server::Rd));
    until(g, "Movement");
    choose(g, Action::Continue);
    until(g, "Breach R&D");
}
#[test]
fn winning_jailbreak_steal_stops_before_rez_and_second_access() {
    let mut g = Match::engineering_turns(winning_position(3)).unwrap();
    jailbreak(&mut g);
    assert_eq!(g.observe(Player::Runner).grip_count, 1);
    let action = g
        .observe(Player::Runner)
        .decision
        .unwrap()
        .choices
        .into_iter()
        .find(|a| a.label() == "Access the next R&D card")
        .unwrap();
    choose(&mut g, action);
    choose(&mut g, Action::StealAgenda);
    let v = g.observe(Player::Runner);
    assert_eq!(v.agenda_points, [0, 6]);
    assert_eq!(v.match_winner, Some(Player::Runner));
    assert!(v.decision.is_none());
    assert!(g.observe(Player::Corp).decision.is_none());
    assert!(!v.servers[0].ice[0].rezzed);
    assert_eq!(v.rd_count, 1);
    assert!(!v.history.iter().any(|s| s.contains("accesses Hedge Fund")));
}
#[test]
fn below_threshold_steal_suspends_for_corp_then_continues_multiaccess() {
    let mut g = Match::engineering_turns(winning_position(2)).unwrap();
    jailbreak(&mut g);
    choose(&mut g, Action::AccessRd);
    choose(&mut g, Action::StealAgenda);
    let v = g.observe(Player::Corp);
    assert_eq!(v.agenda_points, [0, 5]);
    assert!(v.match_winner.is_none());
    assert!(v.phase.starts_with("Send a Message"));
    let d = v.decision.unwrap();
    assert!(
        d.choices
            .iter()
            .any(|a| a.label() == "Decline agenda ability")
    );
    let rez = d
        .choices
        .into_iter()
        .find(|a| a.label() == "Rez installed ICE · free")
        .unwrap();
    choose(&mut g, rez);
    assert_eq!(g.observe(Player::Corp).corp_credits, 0);
    assert!(g.observe(Player::Corp).servers[0].ice[0].rezzed);
    choose(&mut g, Action::AccessRd);
    assert_eq!(
        g.observe(Player::Runner).accessed.unwrap().printing,
        Some("30075")
    );
    choose(&mut g, Action::FinishAccess);
    assert!(g.observe(Player::Runner).run.is_none());
    assert!(
        g.observe(Player::Runner)
            .heap
            .iter()
            .any(|c| c.printing == Some("30028"))
    );
}
#[test]
fn rd_access_does_not_tell_corp_identity_or_card_type() {
    let mut a = winning_position(2);
    a.cards[2].kind = CardKind::SendAMessage;
    let mut b = a.clone();
    b.cards[2].kind = CardKind::Palisade;
    let mut a = Match::engineering_turns(a).unwrap();
    let mut b = Match::engineering_turns(b).unwrap();
    for g in [&mut a, &mut b] {
        jailbreak(g);
        choose(g, Action::AccessRd);
    }
    assert_eq!(a.observe(Player::Corp), b.observe(Player::Corp));
    assert_ne!(a.observe(Player::Runner), b.observe(Player::Runner));
}
#[test]
fn archives_turns_discard_faceup_and_root_trash_becomes_a_new_candidate() {
    let mut p = EngineeringPosition::economy(1);
    p.active = Player::Runner;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::HedgeFund, Zone::Archives),
        EngineeringCard::new(CardKind::OffworldOffice, Zone::Archives),
        installed(CardKind::ManegarmSkunkworks, Zone::Root(Server::Archives)),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    assert!(
        g.observe(Player::Runner)
            .archives
            .iter()
            .all(|c| c.title.is_none() && !c.face_up)
    );
    until(&mut g, "Action phase");
    choose(&mut g, Action::RunServer(Server::Archives));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Breach Archives");
    assert!(
        g.observe(Player::Runner)
            .archives
            .iter()
            .all(|c| c.title.is_some() && c.face_up)
    );
    choose(&mut g, Action::AccessRoot(0));
    choose(&mut g, Action::TrashAccessed);
    choose(&mut g, Action::PayRunCredits(0));
    choose(&mut g, Action::AccessArchives(2));
    assert_eq!(
        g.observe(Player::Runner).decision.unwrap().choices,
        vec![Action::FinishAccess]
    );
    choose(&mut g, Action::FinishAccess);
    choose(&mut g, Action::AccessArchives(1));
    choose(&mut g, Action::StealAgenda);
    choose(&mut g, Action::AccessArchives(0));
    choose(&mut g, Action::FinishAccess);
    assert!(g.observe(Player::Runner).run.is_none());
    assert_eq!(g.observe(Player::Runner).agenda_points, [0, 2]);
}
fn scored_position(kind: CardKind, advancement: u32) -> EngineeringPosition {
    let mut p = EngineeringPosition::economy(4);
    p.shuffle_decks = false;
    p.corp_credits = 0;
    let mut agenda = installed(kind, Zone::Root(Server::Remote(1)));
    agenda.advancement = advancement;
    p.cards = vec![
        agenda,
        EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
        EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Rd),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
    ];
    p
}
#[test]
fn hub_scoring_offers_optional_two_card_draw_at_one_point() {
    let mut g = Match::engineering_turns(scored_position(CardKind::SuperconductingHub, 3)).unwrap();
    let id = g.observe(Player::Corp).servers[3].root[0].instance.unwrap();
    choose(&mut g, Action::Score(id));
    let v = g.observe(Player::Corp);
    assert_eq!(v.agenda_points, [1, 0]);
    assert_eq!(v.hq_count, 0);
    assert!(v.phase.starts_with("Superconducting Hub"));
    let d = v.decision.unwrap();
    assert!(d.choices.contains(&Action::DeclineAgendaAbility));
    let draw = d
        .choices
        .into_iter()
        .find(|a| a.label() == "Draw 2 cards · Superconducting Hub")
        .unwrap();
    choose(&mut g, draw);
    assert_eq!(g.observe(Player::Corp).hq_count, 2);
    assert_eq!(g.observe(Player::Corp).rd_count, 1);
}
#[test]
fn hub_hand_size_applies_only_in_corps_score_area_and_stacks() {
    for (owner, copies, hand_count, expected_discard) in [
        (Player::Corp, 1, 8, 1),
        (Player::Corp, 2, 10, 1),
        (Player::Runner, 1, 7, 2),
    ] {
        let mut p = EngineeringPosition::economy(3);
        p.active = owner;
        p.shuffle_decks = false;
        p.agenda_points[owner.index()] = copies;
        p.cards = (0..copies)
            .map(|_| {
                EngineeringCard::new(
                    CardKind::SuperconductingHub,
                    if owner == Player::Corp {
                        Zone::CorpScore
                    } else {
                        Zone::RunnerScore
                    },
                )
            })
            .collect();
        p.cards.extend((0..hand_count).map(|_| {
            EngineeringCard::new(
                if owner == Player::Corp {
                    CardKind::HedgeFund
                } else {
                    CardKind::SureGamble
                },
                if owner == Player::Corp {
                    Zone::Hq
                } else {
                    Zone::Grip
                },
            )
        }));
        // A Corp start draws one card; use one fewer initial hand card.
        if owner == Player::Corp {
            p.cards.pop();
            p.cards
                .push(EngineeringCard::new(CardKind::HedgeFund, Zone::Rd));
        }
        let mut g = Match::engineering_turns(p).unwrap();
        until(&mut g, "Action phase");
        for _ in 0..if owner == Player::Corp { 3 } else { 4 } {
            choose(&mut g, Action::GainCredit);
            if g.observe(owner).clicks[owner.index()] > 0 {
                until(&mut g, "Action phase");
            }
        }
        until(&mut g, "Choose cards to discard");
        let ids: Vec<_> = g
            .observe(owner)
            .hand
            .iter()
            .take(expected_discard)
            .map(|c| c.instance.unwrap())
            .collect();
        for id in ids {
            choose(&mut g, Action::ToggleDiscard(id));
        }
        assert!(
            g.observe(owner)
                .decision
                .unwrap()
                .choices
                .contains(&Action::ConfirmDiscard)
        );
        choose(&mut g, Action::ConfirmDiscard);
    }
}
#[test]
fn send_a_message_score_uses_same_checkpoint_and_optional_free_rez() {
    for (starting, winning) in [(2, false), (3, true)] {
        let mut p = scored_position(CardKind::SendAMessage, 5);
        p.agenda_points[0] = starting;
        p.cards
            .push(installed(CardKind::Palisade, Zone::Protecting(Server::Hq)));
        let mut g = Match::engineering_turns(p).unwrap();
        let id = g.observe(Player::Corp).servers[3].root[0].instance.unwrap();
        choose(&mut g, Action::Score(id));
        let v = g.observe(Player::Corp);
        assert_eq!(v.agenda_points[0], starting + 3);
        if winning {
            assert_eq!(v.match_winner, Some(Player::Corp));
            assert!(v.decision.is_none());
        } else {
            assert!(v.phase.starts_with("Send a Message"));
            let action = v
                .decision
                .unwrap()
                .choices
                .into_iter()
                .find(|a| matches!(a, Action::RezIceFree(_)))
                .unwrap();
            choose(&mut g, action);
            assert!(g.observe(Player::Corp).servers[0].ice[0].rezzed);
        }
        assert_eq!(g.observe(Player::Corp).corp_credits, 0);
    }
}
#[test]
fn hub_optional_draw_respects_deck_exhaustion_and_lawful_baseline_declines() {
    for cards_left in [0, 1, 2] {
        let mut p = scored_position(CardKind::SuperconductingHub, 3);
        p.cards.truncate(1 + cards_left);
        let mut g = Match::engineering_turns(p).unwrap();
        let id = g.observe(Player::Corp).servers[3].root[0].instance.unwrap();
        choose(&mut g, Action::Score(id));
        let v = g.observe(Player::Corp);
        assert_eq!(
            netrunner_unfolded::game::baseline_response(&v),
            Some(if cards_left < 2 {
                Action::DeclineAgendaAbility
            } else {
                Action::DrawAgendaCards
            })
        );
        let mut declined = Match::resume(g.checkpoint()).unwrap();
        choose(&mut declined, Action::DeclineAgendaAbility);
        assert!(declined.observe(Player::Corp).match_winner.is_none());
        assert_eq!(declined.observe(Player::Corp).rd_count, cards_left);
        choose(&mut g, Action::DrawAgendaCards);
        assert_eq!(
            g.observe(Player::Corp).match_winner,
            if cards_left < 2 {
                Some(Player::Runner)
            } else {
                None
            }
        );
        assert_eq!(g.observe(Player::Corp).hq_count, cards_left.min(2));
    }
}
#[test]
fn new_agendas_require_trashing_old_agenda_before_finishing_installation() {
    for kind in [CardKind::SendAMessage, CardKind::SuperconductingHub] {
        let mut p = scored_position(CardKind::SuperconductingHub, 0);
        p.cards.push(EngineeringCard::new(kind, Zone::Hq));
        let mut g = Match::engineering_turns(p).unwrap();
        until(&mut g, "Action phase");
        let id = g
            .observe(Player::Corp)
            .hand
            .iter()
            .find(|c| c.printing == Some(kind.description().1))
            .unwrap()
            .instance
            .unwrap();
        choose(&mut g, Action::InstallCard(id));
        choose(&mut g, Action::Destination(Server::Remote(1)));
        assert!(
            !g.observe(Player::Corp)
                .decision
                .unwrap()
                .choices
                .contains(&Action::FinishInstall)
        );
        let old = g.observe(Player::Corp).servers[3].root[0].instance.unwrap();
        choose(&mut g, Action::TrashInstalled(old));
        choose(&mut g, Action::FinishInstall);
    }
}
#[test]
fn unsupported_rd_access_is_refused_without_revealing_or_advancing() {
    let mut p = winning_position(2);
    p.cards[2].kind = CardKind::NicoCampaign;
    let mut g = Match::engineering_turns(p).unwrap();
    jailbreak(&mut g);
    let before = g.observe(Player::Runner);
    let d = before.decision.as_ref().unwrap();
    let error = g.choose(d.actor, d.revision, Action::AccessRd).unwrap_err();
    assert!(!error.contains("Nico") && !error.contains("30037"));
    assert_eq!(g.observe(Player::Runner), before);
}
#[test]
fn jailbreak_and_docklands_add_accesses_without_consuming_root_access() {
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.corp_credits = 0;
    p.shuffle_decks = false;
    p.cards = vec![
        EngineeringCard::new(CardKind::Jailbreak, Zone::Grip),
        installed(CardKind::DocklandsPass, Zone::Rig),
        installed(CardKind::ManegarmSkunkworks, Zone::Root(Server::Hq)),
        EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
        EngineeringCard::new(CardKind::Palisade, Zone::Hq),
        EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Hq),
    ];
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    let event = g.observe(Player::Runner).hand[0].instance.unwrap();
    choose(&mut g, Action::PlayCard(event));
    assert_eq!(
        g.observe(Player::Runner).decision.unwrap().choices,
        vec![Action::RunServer(Server::Hq), Action::RunServer(Server::Rd)]
    );
    choose(&mut g, Action::RunServer(Server::Hq));
    until(&mut g, "Movement");
    choose(&mut g, Action::Continue);
    until(&mut g, "Breach HQ");
    assert_eq!(
        g.observe(Player::Runner).run.unwrap().hq_access_remaining,
        Some(3)
    );
    assert_eq!(g.observe(Player::Runner).grip_count, 0); // Empty stack does not end a run or lose.
    choose(&mut g, Action::AccessHq);
    choose(&mut g, Action::FinishAccess);
    choose(&mut g, Action::AccessRoot(0));
    choose(&mut g, Action::FinishAccess);
    assert_eq!(
        g.observe(Player::Runner).run.unwrap().hq_access_remaining,
        Some(2)
    );
    for _ in 0..2 {
        choose(&mut g, Action::AccessHq);
        choose(&mut g, Action::FinishAccess);
    }
    assert!(g.observe(Player::Runner).run.is_none());
    assert!(g.observe(Player::Runner).match_winner.is_none());
}
#[test]
fn rd_multiaccess_moves_down_after_leaving_or_trashing_and_never_repeats() {
    for first in [CardKind::Palisade, CardKind::ManegarmSkunkworks] {
        let mut p = winning_position(2);
        p.cards[2].kind = first;
        let mut g = Match::engineering_turns(p).unwrap();
        jailbreak(&mut g);
        choose(&mut g, Action::AccessRd);
        if first == CardKind::ManegarmSkunkworks {
            choose(&mut g, Action::TrashAccessed);
            choose(&mut g, Action::PayRunCredits(0));
        } else {
            choose(&mut g, Action::FinishAccess);
        }
        choose(&mut g, Action::AccessRd);
        assert_eq!(
            g.observe(Player::Runner).accessed.unwrap().printing,
            Some("30075")
        );
        choose(&mut g, Action::FinishAccess);
        assert!(g.observe(Player::Runner).run.is_none());
    }
}
#[test]
fn empty_centrals_complete_and_hub_steal_has_no_corp_draw() {
    for server in [Server::Hq, Server::Rd, Server::Archives] {
        let mut p = EngineeringPosition::economy(1);
        p.active = Player::Runner;
        p.cards.clear();
        let mut g = Match::engineering_turns(p).unwrap();
        until(&mut g, "Action phase");
        choose(&mut g, Action::RunServer(server));
        until(&mut g, "Movement");
        choose(&mut g, Action::Continue);
        choose(&mut g, Action::PassPriority);
        choose(&mut g, Action::PassPriority);
        assert!(g.observe(Player::Runner).run.is_none());
        assert!(g.observe(Player::Runner).match_winner.is_none());
    }
    let mut p = winning_position(2);
    p.cards[2].kind = CardKind::SuperconductingHub;
    let mut g = Match::engineering_turns(p).unwrap();
    jailbreak(&mut g);
    choose(&mut g, Action::AccessRd);
    choose(&mut g, Action::StealAgenda);
    assert_eq!(g.observe(Player::Runner).agenda_points, [0, 3]);
    assert_eq!(g.observe(Player::Corp).hq_count, 0);
    assert_eq!(
        g.observe(Player::Runner).decision.unwrap().choices,
        vec![Action::AccessRd]
    );
}
#[test]
fn winning_hub_score_precedes_optional_draw_even_with_empty_rd() {
    let mut p = scored_position(CardKind::SuperconductingHub, 3);
    p.agenda_points[0] = 5;
    p.cards.truncate(1);
    let mut g = Match::engineering_turns(p).unwrap();
    let id = g.observe(Player::Corp).servers[3].root[0].instance.unwrap();
    choose(&mut g, Action::Score(id));
    assert_eq!(g.observe(Player::Corp).match_winner, Some(Player::Corp));
    assert!(g.observe(Player::Corp).decision.is_none());
}
#[test]
fn central_worked_positions_retain_the_exact_introductory_decks() {
    let counts = |p: EngineeringPosition| {
        let mut m = std::collections::BTreeMap::new();
        for c in p.cards {
            *m.entry(c.kind.description().1).or_insert(0) += 1;
        }
        m
    };
    for winning in [false, true] {
        assert_eq!(
            counts(EngineeringPosition::central_access(42, winning)),
            counts(EngineeringPosition::hq_access(42))
        );
    }
}
#[test]
fn baseline_advances_send_a_message_from_four_to_five() {
    let mut p = scored_position(CardKind::SendAMessage, 4);
    p.corp_credits = 1;
    let mut g = Match::engineering_turns(p).unwrap();
    until(&mut g, "Action phase");
    let v = g.observe(Player::Corp);
    let id = v.servers[3].root[0].instance.unwrap();
    assert_eq!(
        netrunner_unfolded::game::baseline_response(&v),
        Some(Action::Advance(id))
    );
    choose(&mut g, Action::Advance(id));
    choose(&mut g, Action::Score(id));
    assert_eq!(g.observe(Player::Corp).agenda_points, [3, 0]);
}
#[test]
fn rd_trash_payment_does_not_reveal_the_card_to_corp_before_it_moves() {
    let mut p = winning_position(2);
    p.cards[2].kind = CardKind::ManegarmSkunkworks;
    let mut g = Match::engineering_turns(p).unwrap();
    jailbreak(&mut g);
    choose(&mut g, Action::AccessRd);
    choose(&mut g, Action::TrashAccessed);
    let corp = serde_json::to_string(&g.observe(Player::Corp)).unwrap();
    assert!(!corp.contains("Manegarm") && !corp.contains("30042"));
    choose(&mut g, Action::PayRunCredits(0));
    assert!(
        g.observe(Player::Corp)
            .archives
            .iter()
            .any(|c| c.printing == Some("30042"))
    );
}
#[test]
fn checkpoint_refuses_missing_or_wrong_jailbreak_event_reference() {
    let mut g = Match::engineering_turns(winning_position(2)).unwrap();
    jailbreak(&mut g);
    for id in [999, 2] {
        let mut value = serde_json::to_value(g.checkpoint()).unwrap();
        value["state"]["turn"]["run"]["jailbreak"] = serde_json::json!(id);
        assert!(Match::resume(serde_json::from_value(value).unwrap()).is_err());
    }
}
#[test]
fn suspended_access_and_trash_payment_reject_cards_from_another_server() {
    for payment in [false, true] {
        let mut p = winning_position(2);
        p.cards[2].kind = CardKind::ManegarmSkunkworks;
        let mut g = Match::engineering_turns(p).unwrap();
        jailbreak(&mut g);
        choose(&mut g, Action::AccessRd);
        if payment {
            choose(&mut g, Action::TrashAccessed);
        }
        for zone in ["Hq", "Archives"] {
            let mut value = serde_json::to_value(g.checkpoint()).unwrap();
            value["state"]["cards"][2]["zone"] = serde_json::json!(zone);
            assert!(
                Match::resume(serde_json::from_value(value).unwrap()).is_err(),
                "{zone} payment={payment}"
            );
        }
    }
}
#[test]
fn send_a_message_skips_empty_rez_choices_on_score_and_steal() {
    for rezzed in [false, true] {
        let mut p = winning_position(2);
        if rezzed {
            p.cards.last_mut().unwrap().rezzed = true;
        } else {
            p.cards.pop();
        }
        let mut g = Match::engineering_turns(p).unwrap();
        jailbreak(&mut g);
        choose(&mut g, Action::AccessRd);
        choose(&mut g, Action::StealAgenda);
        assert_eq!(
            g.observe(Player::Runner).decision.unwrap().choices,
            vec![Action::AccessRd]
        );
        assert!(g.observe(Player::Corp).decision.is_none());
        let mut p = scored_position(CardKind::SendAMessage, 5);
        if rezzed {
            let mut ice = installed(CardKind::Palisade, Zone::Protecting(Server::Hq));
            ice.rezzed = true;
            p.cards.push(ice);
        }
        let mut g = Match::engineering_turns(p).unwrap();
        let id = g.observe(Player::Corp).servers[3].root[0].instance.unwrap();
        choose(&mut g, Action::Score(id));
        assert!(!g.observe(Player::Corp).phase.starts_with("Send a Message"));
        assert!(
            !g.observe(Player::Corp)
                .decision
                .unwrap()
                .choices
                .contains(&Action::DeclineAgendaAbility)
        );
    }
}
#[test]
fn jailbreak_completion_does_not_claim_an_overclock_resource() {
    let mut p = winning_position(2);
    p.cards.pop();
    let mut g = Match::engineering_turns(p).unwrap();
    jailbreak(&mut g);
    choose(&mut g, Action::AccessRd);
    choose(&mut g, Action::StealAgenda);
    choose(&mut g, Action::AccessRd);
    choose(&mut g, Action::FinishAccess);
    assert!(
        !g.observe(Player::Runner)
            .history
            .iter()
            .any(|s| s.contains("Overclock"))
    );
}
