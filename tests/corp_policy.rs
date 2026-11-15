use netrunner_unfolded::{
    game::{Action, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone},
    policies::Policy,
};

fn choose(game: &mut Match, actor: Player, action: Action) {
    let revision = game.observe(actor).decision.unwrap().revision;
    game.choose(actor, revision, action).unwrap();
}

fn action_window(game: &mut Match) {
    for _ in 0..8 {
        if game
            .observe(Player::Corp)
            .decision
            .as_ref()
            .is_some_and(|d| d.choices.contains(&Action::GainCredit))
        {
            return;
        }
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|p| game.observe(*p).decision.is_some())
            .unwrap();
        choose(game, actor, Action::PassPriority);
    }
    panic!("Corp action window did not arrive");
}

#[test]
fn affordable_ice_protects_hq_agendas_before_clicking_for_an_unneeded_credit() {
    // Corp guide: HQ agendas are vulnerable to access; installing the first ICE
    // costs zero and Palisade costs three to rez. Five credits afford this defense.
    let mut position = EngineeringPosition::economy(3);
    position.cards.retain(|c| c.zone != Zone::Hq);
    position.cards.extend([
        EngineeringCard::new(CardKind::Palisade, Zone::Hq),
        EngineeringCard::new(CardKind::SendAMessage, Zone::Hq),
    ]);
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let ice = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30072"))
        .unwrap()
        .instance
        .unwrap();
    assert_eq!(Policy::CorpV1.choose(&view), Some(Action::InstallCard(ice)));
    choose(&mut game, Player::Corp, Action::InstallCard(ice));
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::Destination(Server::Hq))
    );
}

#[test]
fn complete_an_office_this_turn_then_score_before_spending_on_defense() {
    let mut position = EngineeringPosition::economy(42);
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 3,
        installed_turn: Some(1),
        rezzed: false,
    });
    position
        .cards
        .push(EngineeringCard::new(CardKind::Palisade, Zone::Hq));
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let office = view
        .servers
        .iter()
        .flat_map(|s| &s.root)
        .find(|c| c.printing == Some("30067"))
        .unwrap()
        .instance
        .unwrap();
    assert_eq!(Policy::CorpV1.choose(&view), Some(Action::Advance(office)));
    choose(&mut game, Player::Corp, Action::Advance(office));
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::Score(office))
    );
    choose(&mut game, Player::Corp, Action::Score(office));
    assert_eq!(game.observe(Player::Corp).agenda_points, [2, 0]);
    assert_eq!(game.observe(Player::Corp).corp_credits, 11);
}

#[test]
fn hub_declines_a_draw_that_would_empty_rd_before_the_next_mandatory_draw() {
    let mut position = EngineeringPosition::economy(42);
    position.cards.retain(|c| c.zone != Zone::Rd);
    position.cards.extend([
        EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
        EngineeringCard::new(CardKind::Palisade, Zone::Rd),
    ]);
    position.cards.push(EngineeringCard {
        kind: CardKind::SuperconductingHub,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 3,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut game = Match::engineering_turns(position).unwrap();
    let hub = game.observe(Player::Corp).servers[3].root[0]
        .instance
        .unwrap();
    choose(&mut game, Player::Corp, Action::Score(hub));
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::DeclineAgendaAbility)
    );
}

#[test]
fn economy_and_installation_form_a_legal_scoring_plan_across_turns() {
    let mut game = Match::engineering_turns(EngineeringPosition::economy(42)).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let hedge = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30075"))
        .unwrap()
        .instance
        .unwrap();
    assert_eq!(Policy::CorpV1.choose(&view), Some(Action::PlayCard(hedge)));
    for _ in 0..100 {
        if game.observe(Player::Corp).agenda_points[0] >= 2 {
            return;
        }
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|p| game.observe(*p).decision.is_some())
            .unwrap();
        let view = game.observe(actor);
        let action = if actor == Player::Corp {
            Policy::CorpV1.choose(&view).expect(view.phase)
        } else if view
            .decision
            .as_ref()
            .unwrap()
            .choices
            .contains(&Action::GainCredit)
        {
            Action::GainCredit
        } else {
            Action::PassPriority
        };
        choose(&mut game, actor, action);
    }
    panic!("the available Office was never scored");
}

#[test]
fn admitted_pending_choices_always_have_an_explicit_corp_response() {
    // Existing source-derived hostile inputs: Brân replacement, ICE payments,
    // damage/access, scoring rewards, campaign income and discarding.
    for position in [
        EngineeringPosition::hq_access(3),
        EngineeringPosition::turn_income(42),
        EngineeringPosition::run_income(89, true),
    ] {
        let mut game = Match::engineering_turns(position).unwrap();
        for _ in 0..180 {
            if game.observe(Player::Corp).match_winner.is_some() {
                break;
            }
            let actor = [Player::Corp, Player::Runner]
                .into_iter()
                .find(|p| game.observe(*p).decision.is_some())
                .unwrap();
            let view = game.observe(actor);
            let action = if actor == Player::Corp {
                Policy::CorpV1
            } else {
                Policy::RunnerV0
            }
            .choose(&view)
            .unwrap_or_else(|| panic!("no response to {}: {:?}", view.phase, view.decision));
            choose(&mut game, actor, action);
        }
    }
}

#[test]
fn corp_rezzes_the_approached_defense_and_uses_brans_free_install_without_trashing_it() {
    let mut position = EngineeringPosition::run_income(89, true);
    position.corp_credits = 9; // Brân costs six; retain three for Palisade.
    for c in &mut position.cards {
        if c.kind == CardKind::Bran {
            c.rezzed = false;
        }
    }
    let mut game = Match::engineering_turns(position).unwrap();
    for _ in 0..4 {
        let actor = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|p| game.observe(*p).decision.is_some())
            .unwrap();
        choose(&mut game, actor, Action::PassPriority);
    }
    choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
    let mut rezzed = false;
    for _ in 0..20 {
        let actor = [Player::Runner, Player::Corp]
            .into_iter()
            .find(|p| game.observe(*p).decision.is_some())
            .unwrap();
        let view = game.observe(actor);
        let choices = &view.decision.as_ref().unwrap().choices;
        if let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(_))) {
            assert_eq!(Policy::CorpV1.choose(&view), Some(*action));
            choose(&mut game, actor, *action);
            rezzed = true;
        } else if let Some(action) = choices
            .iter()
            .find(|a| matches!(a, Action::InstallInward(_)))
        {
            assert!(rezzed);
            assert_eq!(Policy::CorpV1.choose(&view), Some(*action));
            choose(&mut game, actor, *action);
            assert_eq!(
                Policy::CorpV1.choose(&game.observe(Player::Corp)),
                Some(Action::FinishInstall)
            );
            return;
        } else {
            choose(&mut game, actor, Action::PassPriority);
        }
    }
    panic!("Brân encounter did not suspend at its installation");
}

#[test]
fn ordinary_setup_and_opening_decisions_use_the_named_policy_without_fallback() {
    let mut game = Match::introductory(3);
    for _ in 0..200 {
        if game.observe(Player::Corp).match_winner.is_some() {
            break;
        }
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|p| game.observe(*p).decision.is_some())
            .unwrap();
        let view = game.observe(actor);
        let action = if actor == Player::Corp {
            Policy::CorpV1
        } else {
            Policy::RunnerV0
        }
        .choose(&view)
        .unwrap_or_else(|| panic!("no policy response to {}: {:?}", view.phase, view.decision));
        choose(&mut game, actor, action);
    }
}

#[test]
fn cheap_defense_covers_rd_after_hq_and_preserves_its_rez_reserve() {
    let mut position = EngineeringPosition::economy(42);
    position.corp_credits = 2;
    position.cards.retain(|c| c.zone != Zone::Hq);
    position.cards.extend([
        EngineeringCard::new(CardKind::Tithe, Zone::Hq),
        EngineeringCard::new(CardKind::Bran, Zone::Hq),
        EngineeringCard {
            kind: CardKind::NicoCampaign,
            zone: Zone::Root(Server::Remote(1)),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: false,
        },
        EngineeringCard {
            kind: CardKind::Palisade,
            zone: Zone::Protecting(Server::Hq),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: true,
        },
    ]);
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let tithe = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30073"))
        .unwrap()
        .instance
        .unwrap();
    assert_eq!(
        Policy::CorpV1.choose(&view),
        Some(Action::InstallCard(tithe))
    );
    choose(&mut game, Player::Corp, Action::InstallCard(tithe));
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::Destination(Server::Rd))
    );
    choose(&mut game, Player::Corp, Action::Destination(Server::Rd));
    choose(&mut game, Player::Corp, Action::FinishInstall);
    // Spending both credits on Nico now would leave Tithe unaffordable.
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::PassPriority)
    );
}

#[test]
fn rezzing_and_using_economy_assets_respects_the_board_and_click_opportunity() {
    let mut position = EngineeringPosition::economy(42);
    position.corp_credits = 4;
    position.cards.retain(|c| c.zone != Zone::Hq);
    position.cards.push(EngineeringCard::new(
        CardKind::RegolithMiningLicense,
        Zone::Hq,
    ));
    position
        .cards
        .push(EngineeringCard::new(CardKind::NicoCampaign, Zone::Hq));
    position
        .cards
        .extend((0..6).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Rd)));
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let nico = game
        .observe(Player::Corp)
        .hand
        .iter()
        .find(|c| c.printing == Some("30037"))
        .unwrap()
        .instance
        .unwrap();
    choose(&mut game, Player::Corp, Action::InstallCard(nico));
    choose(
        &mut game,
        Player::Corp,
        Action::Destination(Server::NewRemote),
    );
    choose(&mut game, Player::Corp, Action::FinishInstall);
    let view = game.observe(Player::Corp);
    assert_eq!(Policy::CorpV1.choose(&view), Some(Action::Rez(nico)));
    let initial_turn = view.turn_number;
    choose(&mut game, Player::Corp, Action::Rez(nico));
    // A sole mandatory income resolves automatically at the next Corp turn.
    for _ in 0..100 {
        let view = game.observe(Player::Corp);
        if view.servers[3].root[0].hosted_credits == 6 {
            assert_eq!(view.turn_number, initial_turn + 2);
            return;
        }
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|p| game.observe(*p).decision.is_some())
            .unwrap();
        let view = game.observe(actor);
        let choices = &view.decision.as_ref().unwrap().choices;
        let action = if choices.contains(&Action::GainCredit) {
            Action::GainCredit
        } else if choices.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            if actor == Player::Corp {
                Policy::CorpV1
            } else {
                Policy::RunnerV0
            }
            .choose(&view)
            .unwrap()
        };
        choose(&mut game, actor, action);
    }
    panic!("next Corp turn was not reached");
}

#[test]
fn regolith_is_installed_rezzed_and_used_instead_of_single_credit_clicks() {
    let mut position = EngineeringPosition::economy(42);
    position.corp_credits = 3;
    position.cards.retain(|c| c.zone != Zone::Hq);
    position.cards.push(EngineeringCard::new(
        CardKind::RegolithMiningLicense,
        Zone::Hq,
    ));
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let regolith = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30071"))
        .unwrap()
        .instance
        .unwrap();
    for expected in [
        Action::InstallCard(regolith),
        Action::Destination(Server::NewRemote),
        Action::FinishInstall,
        Action::Rez(regolith),
    ] {
        assert_eq!(
            Policy::CorpV1.choose(&game.observe(Player::Corp)),
            Some(expected)
        );
        choose(&mut game, Player::Corp, expected);
    }
    action_window(&mut game);
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::TakeRegolith(regolith))
    );
}

#[test]
fn seamless_finishes_an_eligible_office_for_one_credit_and_one_click() {
    let mut position = EngineeringPosition::economy(42);
    position.corp_credits = 2;
    position.cards.retain(|c| c.zone != Zone::Hq);
    position
        .cards
        .push(EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Hq));
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 2,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let seamless = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30040"))
        .unwrap()
        .instance
        .unwrap();
    let office = view.servers[3].root[0].instance.unwrap();
    for expected in [
        Action::PlayCard(seamless),
        Action::SelectTarget(office),
        Action::Score(office),
    ] {
        assert_eq!(
            Policy::CorpV1.choose(&game.observe(Player::Corp)),
            Some(expected)
        );
        choose(&mut game, Player::Corp, expected);
    }
    assert_eq!(game.observe(Player::Corp).corp_credits, 8);
    assert_eq!(game.observe(Player::Corp).clicks[0], 2);
}

#[test]
fn winning_agenda_precedes_a_smaller_scoring_reward() {
    let mut position = EngineeringPosition::economy(42);
    position.agenda_points = [4, 0];
    for (kind, server, counters) in [
        (CardKind::SuperconductingHub, 1, 3),
        (CardKind::OffworldOffice, 2, 4),
    ] {
        position.cards.push(EngineeringCard {
            kind,
            zone: Zone::Root(Server::Remote(server)),
            advancement: counters,
            installed_turn: Some(1),
            rezzed: false,
        });
    }
    let game = Match::engineering_turns(position).unwrap();
    let view = game.observe(Player::Corp);
    let office = view
        .servers
        .iter()
        .flat_map(|s| &s.root)
        .find(|c| c.printing == Some("30067"))
        .unwrap()
        .instance
        .unwrap();
    assert_eq!(Policy::CorpV1.choose(&view), Some(Action::Score(office)));
}

#[test]
fn suspended_replacement_finishes_by_trashing_the_displaced_root_card() {
    let mut position = EngineeringPosition::economy(42);
    position.cards.push(EngineeringCard {
        kind: CardKind::UrticaCipher,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 0,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let view = game.observe(Player::Corp);
    let office = view
        .hand
        .iter()
        .find(|c| c.printing == Some("30067"))
        .unwrap()
        .instance
        .unwrap();
    let urtica = view.servers[3].root[0].instance.unwrap();
    choose(&mut game, Player::Corp, Action::InstallCard(office));
    choose(
        &mut game,
        Player::Corp,
        Action::Destination(Server::Remote(1)),
    );
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::TrashInstalled(urtica))
    );
    choose(&mut game, Player::Corp, Action::TrashInstalled(urtica));
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::FinishInstall)
    );
}

#[test]
fn new_agenda_timing_is_public_and_seamless_does_not_spend_on_an_ineligible_target() {
    let mut position = EngineeringPosition::economy(42);
    position.corp_credits = 2;
    position
        .cards
        .push(EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Hq));
    position.cards.push(EngineeringCard {
        kind: CardKind::Tithe,
        zone: Zone::Protecting(Server::Hq),
        advancement: 0,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let office = game
        .observe(Player::Corp)
        .hand
        .iter()
        .find(|c| c.printing == Some("30067"))
        .unwrap()
        .instance
        .unwrap();
    choose(&mut game, Player::Corp, Action::InstallCard(office));
    choose(
        &mut game,
        Player::Corp,
        Action::Destination(Server::NewRemote),
    );
    choose(&mut game, Player::Corp, Action::FinishInstall);
    action_window(&mut game);
    for role in [Player::Corp, Player::Runner] {
        let view = game.observe(role);
        assert!(view.servers[3].root[0].installed_this_turn);
        assert!(!view.servers[0].ice[0].installed_this_turn);
        assert!(view.hand.iter().all(|c| !c.installed_this_turn));
    }
    let view = game.observe(Player::Corp);
    assert!(
        view.decision
            .as_ref()
            .unwrap()
            .choices
            .iter()
            .any(|a| matches!(a, Action::PlayCard(_)))
    );
    assert_eq!(Policy::CorpV1.choose(&view), Some(Action::Advance(office)));
}

#[test]
fn manegarm_is_placed_with_the_agenda_that_cannot_be_finished_this_turn() {
    let mut position = EngineeringPosition::economy(42);
    position.shuffle_decks = false;
    position
        .cards
        .retain(|c| !matches!(c.zone, Zone::Hq | Zone::Rd));
    position.cards.extend([
        EngineeringCard::new(CardKind::ManegarmSkunkworks, Zone::Hq),
        EngineeringCard::new(CardKind::GovernmentSubsidy, Zone::Rd),
        EngineeringCard {
            kind: CardKind::OffworldOffice,
            zone: Zone::Root(Server::Remote(1)),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: false,
        },
    ]);
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let upgrade = game
        .observe(Player::Corp)
        .hand
        .iter()
        .find(|c| c.printing == Some("30042"))
        .unwrap()
        .instance
        .unwrap();
    for expected in [
        Action::InstallCard(upgrade),
        Action::Destination(Server::Remote(1)),
        Action::FinishInstall,
    ] {
        assert_eq!(
            Policy::CorpV1.choose(&game.observe(Player::Corp)),
            Some(expected)
        );
        choose(&mut game, Player::Corp, expected);
    }
}

#[test]
fn an_existing_protected_empty_remote_is_used_before_building_another_server() {
    let mut position = EngineeringPosition::economy(42);
    position.cards.retain(|c| c.zone != Zone::Hq);
    position.cards.extend([
        EngineeringCard::new(CardKind::OffworldOffice, Zone::Hq),
        EngineeringCard::new(CardKind::Palisade, Zone::Hq),
    ]);
    for server in [Server::Hq, Server::Rd, Server::Remote(1)] {
        position.cards.push(EngineeringCard {
            kind: CardKind::Palisade,
            zone: Zone::Protecting(server),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: true,
        });
    }
    let mut game = Match::engineering_turns(position).unwrap();
    action_window(&mut game);
    let office = game
        .observe(Player::Corp)
        .hand
        .iter()
        .find(|c| c.printing == Some("30067"))
        .unwrap()
        .instance
        .unwrap();
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::InstallCard(office))
    );
    choose(&mut game, Player::Corp, Action::InstallCard(office));
    assert_eq!(
        Policy::CorpV1.choose(&game.observe(Player::Corp)),
        Some(Action::Destination(Server::Remote(1)))
    );
}

#[test]
fn an_empty_hand_draws_toward_a_scoring_plan_only_after_economy_is_available() {
    for (credits, expected) in [(8, Action::Draw), (0, Action::GainCredit)] {
        let mut position = EngineeringPosition::economy(42);
        position.shuffle_decks = false;
        position.corp_credits = credits;
        position
            .cards
            .retain(|c| !matches!(c.zone, Zone::Hq | Zone::Rd));
        position
            .cards
            .extend((0..6).map(|_| EngineeringCard::new(CardKind::GovernmentSubsidy, Zone::Rd)));
        let mut game = Match::engineering_turns(position).unwrap();
        action_window(&mut game);
        assert_eq!(
            Policy::CorpV1.choose(&game.observe(Player::Corp)),
            Some(expected)
        );
    }
}

#[test]
fn brans_free_install_reserves_the_tread_lightly_surcharge_for_later_rez() {
    for (credits, install) in [(12, false), (15, true)] {
        let mut position = EngineeringPosition::run_income(89, true);
        position.corp_credits = credits;
        for c in &mut position.cards {
            if c.kind == CardKind::Bran {
                c.rezzed = false;
            }
        }
        let mut game = Match::engineering_turns(position).unwrap();
        for _ in 0..4 {
            let actor = [Player::Runner, Player::Corp]
                .into_iter()
                .find(|p| game.observe(*p).decision.is_some())
                .unwrap();
            choose(&mut game, actor, Action::PassPriority);
        }
        let tread = game
            .observe(Player::Runner)
            .hand
            .iter()
            .find(|c| c.printing == Some("30012"))
            .unwrap()
            .instance
            .unwrap();
        choose(&mut game, Player::Runner, Action::PlayCard(tread));
        choose(&mut game, Player::Runner, Action::RunServer(Server::Hq));
        let mut checked = false;
        for _ in 0..20 {
            let actor = [Player::Runner, Player::Corp]
                .into_iter()
                .find(|p| game.observe(*p).decision.is_some())
                .unwrap();
            let view = game.observe(actor);
            let choices = &view.decision.as_ref().unwrap().choices;
            if let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(_))) {
                choose(&mut game, actor, *action);
            } else if let Some(action) = choices
                .iter()
                .find(|a| matches!(a, Action::InstallInward(_)))
            {
                assert_eq!(
                    Policy::CorpV1.choose(&view),
                    Some(if install {
                        *action
                    } else {
                        Action::DeclineInstall
                    })
                );
                checked = true;
                break;
            } else {
                choose(&mut game, actor, Action::PassPriority);
            }
        }
        assert!(checked);
    }
}
