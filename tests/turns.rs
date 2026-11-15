use netrunner_unfolded::game::{
    Action, CardId, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
};

fn choose(game: &mut Match, player: Player, action: Action) {
    let decision = game.observe(player).decision.expect("acting player");
    game.choose(player, decision.revision, action).unwrap();
}

fn pass_window(game: &mut Match) {
    for _ in 0..2 {
        let player = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        choose(game, player, Action::PassPriority);
    }
}

fn id(game: &Match, player: Player, title: &str) -> CardId {
    let view = game.observe(player);
    view.hand
        .iter()
        .chain(&view.rig)
        .chain(
            view.servers
                .iter()
                .flat_map(|s| s.root.iter().chain(&s.ice)),
        )
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}

// Pinned Office + Seamless text and CR 1.17 / 10.3: placement is not
// advancement; scoring resolves gain-seven only below the six-point threshold.
#[test]
fn seamless_places_two_then_office_scores_in_a_paid_window_without_a_click() {
    let mut position = EngineeringPosition::economy(42);
    position.corp_credits = 0;
    position.shuffle_decks = false;
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 2,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    choose(&mut game, Player::Corp, Action::GainCredit);
    pass_window(&mut game);
    let seamless = id(&game, Player::Corp, "Seamless Launch");
    choose(&mut game, Player::Corp, Action::PlayCard(seamless));
    let office = game
        .observe(Player::Corp)
        .servers
        .iter()
        .flat_map(|s| &s.root)
        .find_map(|c| c.instance)
        .unwrap();
    choose(&mut game, Player::Corp, Action::SelectTarget(office));
    choose(&mut game, Player::Corp, Action::Score(office));
    let view = game.observe(Player::Corp);
    assert_eq!(view.agenda_points, [2, 0]);
    assert_eq!(view.corp_credits, 7);
    assert_eq!(view.clicks, [1, 0]);
    assert_eq!(view.corp_score[0].advancement, 0);
}

#[test]
fn invalid_suspended_turn_targets_refuse_before_the_controller_can_use_them() {
    let game = Match::engineering_turns(EngineeringPosition::economy(42)).unwrap();
    let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
    checkpoint["state"]["pending"] =
        serde_json::json!({"InstallTrash":{"actor":"Corp","card":999,"server":"Hq"}});
    assert!(Match::resume(serde_json::from_value(checkpoint).unwrap()).is_err());
}

#[test]
fn pre_draw_scoring_wins_before_office_reward_but_passing_loses_to_empty_rd() {
    let mut position = EngineeringPosition::economy(1);
    position.cards.retain(|c| c.zone != Zone::Rd);
    position.agenda_points = [4, 0];
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 4,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut score = Match::engineering_turns(position.clone()).unwrap();
    let office = score.observe(Player::Corp).servers[3].root[0]
        .instance
        .unwrap();
    choose(&mut score, Player::Corp, Action::Score(office));
    assert_eq!(score.observe(Player::Corp).match_winner, Some(Player::Corp));
    assert_eq!(score.observe(Player::Corp).corp_credits, 5);
    let mut pass = Match::engineering_turns(position).unwrap();
    pass_window(&mut pass);
    assert_eq!(
        pass.observe(Player::Runner).match_winner,
        Some(Player::Runner)
    );
}

#[test]
fn ordinary_turns_discard_selected_cards_together_then_runner_gets_four_clicks_without_draw() {
    let mut position = EngineeringPosition::economy(2);
    position.shuffle_decks = false;
    position
        .cards
        .extend((0..4).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Hq)));
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    // Purging is legal even when there are no virus counters (CR 5.2.6).
    choose(&mut game, Player::Corp, Action::Purge);
    pass_window(&mut game);
    let before = game.observe(Player::Corp);
    assert_eq!(before.hq_count, 7);
    for id in before.hand.iter().take(2).map(|c| c.instance.unwrap()) {
        choose(&mut game, Player::Corp, Action::ToggleDiscard(id));
    }
    assert_eq!(game.observe(Player::Corp).hq_count, 7);
    let saved = game.checkpoint();
    game = Match::resume(saved).unwrap();
    choose(&mut game, Player::Corp, Action::ConfirmDiscard);
    assert_eq!(game.observe(Player::Corp).hq_count, 5);
    assert!(
        game.observe(Player::Runner)
            .archives
            .iter()
            .all(|c| c.title.is_none())
    );
    pass_window(&mut game);
    pass_window(&mut game);
    pass_window(&mut game);
    let runner = game.observe(Player::Runner);
    assert_eq!(runner.clicks, [0, 4]);
    assert_eq!(runner.grip_count, 4);
}

#[test]
fn program_install_can_trash_extra_programs_and_duplicate_unique_keeps_newest() {
    let mut position = EngineeringPosition::economy(3);
    position.active = Player::Runner;
    position.runner_credits = 20;
    for kind in [
        CardKind::Cleaver,
        CardKind::Cleaver,
        CardKind::Unity,
        CardKind::Unity,
        CardKind::DocklandsPass,
    ] {
        position.cards.push(EngineeringCard {
            kind,
            zone: Zone::Rig,
            advancement: 0,
            installed_turn: Some(1),
            rezzed: false,
        });
    }
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    let cleaver = id(&game, Player::Runner, "Cleaver");
    choose(&mut game, Player::Runner, Action::InstallCard(cleaver));
    assert!(
        !game
            .observe(Player::Runner)
            .decision
            .unwrap()
            .choices
            .contains(&Action::FinishInstall)
    );
    let old: Vec<_> = game
        .observe(Player::Runner)
        .rig
        .iter()
        .filter(|c| c.printing == Some("30026"))
        .map(|c| c.instance.unwrap())
        .collect();
    for card in old {
        choose(&mut game, Player::Runner, Action::TrashInstalled(card));
    }
    choose(&mut game, Player::Runner, Action::FinishInstall);
    assert_eq!(game.observe(Player::Runner).memory_used, 3);
    pass_window(&mut game);
    let new_pass = id(&game, Player::Runner, "Docklands Pass");
    choose(&mut game, Player::Runner, Action::InstallCard(new_pass));
    assert_eq!(
        game.observe(Player::Runner).decision.unwrap().choices,
        vec![Action::FinishInstall]
    );
    choose(&mut game, Player::Runner, Action::FinishInstall);
    let view = game.observe(Player::Runner);
    assert_eq!(
        view.rig
            .iter()
            .filter(|c| c.printing == Some("30013"))
            .count(),
        1
    );
    assert_eq!(
        view.rig
            .iter()
            .find(|c| c.printing == Some("30013"))
            .unwrap()
            .instance,
        Some(new_pass)
    );
    assert_eq!(
        view.heap
            .iter()
            .filter(|c| c.printing == Some("30013"))
            .count(),
        1
    );
}

#[test]
fn accessed_office_is_stolen_without_the_corp_scoring_reward() {
    let mut position = EngineeringPosition::economy(8);
    position.active = Player::Runner;
    position.agenda_points = [0, 4];
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 2,
        installed_turn: Some(1),
        rezzed: false,
    });
    position.accessed_office = true;
    let mut game = Match::engineering_turns(position).unwrap();
    choose(&mut game, Player::Runner, Action::StealAgenda);
    let view = game.observe(Player::Runner);
    assert_eq!(view.match_winner, Some(Player::Runner));
    assert_eq!(view.agenda_points, [0, 6]);
    assert_eq!(view.corp_credits, 5);
    assert_eq!(view.runner_credits, 5);
    assert_eq!(view.runner_score[0].advancement, 0);
}

#[test]
fn baseline_completes_pending_discard_from_its_lawful_projection() {
    let mut position = EngineeringPosition::economy(2);
    position.shuffle_decks = false;
    position
        .cards
        .extend((0..4).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Hq)));
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    choose(&mut game, Player::Corp, Action::Purge);
    pass_window(&mut game);
    for _ in 0..3 {
        let view = game.observe(Player::Corp);
        choose(
            &mut game,
            Player::Corp,
            netrunner_unfolded::game::baseline_response(&view).unwrap(),
        );
    }
    assert_eq!(game.observe(Player::Corp).hq_count, 5);
}

#[test]
fn turns_native_load_requires_existing_correct_family_without_writing() {
    use netrunner_unfolded::play::{Admission, Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turns.json");
    let mut play = Play::load_turns(
        SaveOptions {
            path: path.clone(),
            recover_from: None,
        },
        Player::Runner,
    );
    play.admit();
    assert!(matches!(play.admission(), Admission::Refused(_)));
    assert!(!path.exists());
    let mut bran = Play::saved(&path);
    bran.admit();
    drop(bran);
    let before = std::fs::read(&path).unwrap();
    let mut play = Play::load_turns(
        SaveOptions {
            path: path.clone(),
            recover_from: None,
        },
        Player::Runner,
    );
    play.admit();
    assert!(matches!(play.admission(), Admission::Refused(_)));
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn replacing_a_remote_root_preserves_its_server_even_while_empty() {
    let mut position = EngineeringPosition::economy(10);
    position.cards.push(EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 0,
        installed_turn: Some(1),
        rezzed: false,
    });
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    let hand = id(&game, Player::Corp, "Offworld Office");
    let old = game.observe(Player::Corp).servers[3].root[0]
        .instance
        .unwrap();
    choose(&mut game, Player::Corp, Action::InstallCard(hand));
    choose(
        &mut game,
        Player::Corp,
        Action::Destination(Server::Remote(1)),
    );
    assert!(
        !game
            .observe(Player::Corp)
            .decision
            .unwrap()
            .choices
            .contains(&Action::FinishInstall)
    );
    choose(&mut game, Player::Corp, Action::TrashInstalled(old));
    assert!(
        game.observe(Player::Runner)
            .servers
            .iter()
            .any(|s| s.server == Server::Remote(1))
    );
    choose(&mut game, Player::Corp, Action::FinishInstall);
    assert_eq!(
        game.observe(Player::Corp).servers[3].root[0].instance,
        Some(hand)
    );
    assert!(game.observe(Player::Runner).archives[0].title.is_none());
}

#[test]
fn ice_install_trashes_only_same_server_before_calculating_cost_and_finishes_outermost() {
    let mut position = EngineeringPosition::economy(12);
    position.corp_credits = 1;
    position
        .cards
        .push(EngineeringCard::new(CardKind::Palisade, Zone::Hq));
    for server in [Server::Hq, Server::Hq, Server::Hq, Server::Rd] {
        position.cards.push(EngineeringCard {
            kind: CardKind::Palisade,
            zone: Zone::Protecting(server),
            advancement: 0,
            installed_turn: Some(1),
            rezzed: true,
        });
    }
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    let new = id(&game, Player::Corp, "Palisade");
    let old: Vec<_> = game.observe(Player::Corp).servers[0]
        .ice
        .iter()
        .take(2)
        .map(|c| c.instance.unwrap())
        .collect();
    let rd = game.observe(Player::Corp).servers[1].ice[0]
        .instance
        .unwrap();
    choose(&mut game, Player::Corp, Action::InstallCard(new));
    choose(&mut game, Player::Corp, Action::Destination(Server::Hq));
    assert!(
        !game
            .observe(Player::Corp)
            .decision
            .unwrap()
            .choices
            .contains(&Action::TrashInstalled(rd))
    );
    for card in old {
        choose(&mut game, Player::Corp, Action::TrashInstalled(card));
    }
    choose(&mut game, Player::Corp, Action::FinishInstall);
    let view = game.observe(Player::Corp);
    assert_eq!(view.corp_credits, 0);
    assert_eq!(view.servers[0].ice.last().unwrap().instance, Some(new));
    assert!(
        game.observe(Player::Runner)
            .archives
            .iter()
            .all(|c| c.title.is_some())
    );
}

#[test]
fn economy_cards_require_upfront_costs_and_resolve_net_gains() {
    for (actor, kind, credits, expected) in [
        (Player::Runner, CardKind::SureGamble, 5, 9),
        (Player::Corp, CardKind::HedgeFund, 5, 9),
        (Player::Corp, CardKind::GovernmentSubsidy, 10, 15),
    ] {
        for affordable in [false, true] {
            let mut position = EngineeringPosition::economy(3);
            position.active = actor;
            position.corp_credits = credits - u32::from(!affordable);
            position.runner_credits = position.corp_credits;
            position.cards.push(EngineeringCard::new(
                kind,
                if actor == Player::Corp {
                    Zone::Hq
                } else {
                    Zone::Grip
                },
            ));
            let mut game = Match::engineering_turns(position).unwrap();
            pass_window(&mut game);
            pass_window(&mut game);
            let card = id(&game, actor, kind.description().0);
            let action = Action::PlayCard(card);
            if affordable {
                choose(&mut game, actor, action);
                let view = game.observe(actor);
                assert_eq!(
                    if actor == Player::Corp {
                        view.corp_credits
                    } else {
                        view.runner_credits
                    },
                    expected
                );
            } else {
                let before = game.observe(actor);
                assert!(!before.decision.as_ref().unwrap().choices.contains(&action));
                assert!(
                    game.choose(actor, before.decision.as_ref().unwrap().revision, action)
                        .is_err()
                );
                assert_eq!(game.observe(actor), before);
            }
        }
    }
}

#[test]
fn seamless_can_place_on_nonadvanceable_and_runner_cards_but_not_new_corp_cards() {
    let mut position = EngineeringPosition::economy(42);
    position.shuffle_decks = false;
    for (kind, zone, age) in [
        (CardKind::Palisade, Zone::Protecting(Server::Hq), 1),
        (CardKind::Unity, Zone::Rig, 2),
        (CardKind::OffworldOffice, Zone::Root(Server::Remote(1)), 2),
    ] {
        position.cards.push(EngineeringCard {
            kind,
            zone,
            advancement: 0,
            installed_turn: Some(age),
            rezzed: false,
        });
    }
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    let seamless = id(&game, Player::Corp, "Seamless Launch");
    choose(&mut game, Player::Corp, Action::PlayCard(seamless));
    let view = game.observe(Player::Corp);
    let palisade = view.servers[0].ice[0].instance.unwrap();
    let unity = view.rig[0].instance.unwrap();
    let office = view.servers[3].root[0].instance.unwrap();
    let choices = &view.decision.unwrap().choices;
    assert!(
        choices.contains(&Action::SelectTarget(palisade))
            && choices.contains(&Action::SelectTarget(unity))
    );
    assert!(!choices.contains(&Action::SelectTarget(office)));
    choose(&mut game, Player::Corp, Action::SelectTarget(palisade));
    assert_eq!(game.observe(Player::Corp).servers[0].ice[0].advancement, 2);
    pass_window(&mut game);
    assert!(
        !game
            .observe(Player::Corp)
            .decision
            .unwrap()
            .choices
            .contains(&Action::Advance(palisade))
    );
}

#[test]
fn seamless_with_no_eligible_target_is_still_legal_and_pays_its_cost() {
    let mut position = EngineeringPosition::economy(42);
    position.shuffle_decks = false;
    position.corp_credits = 1;
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    pass_window(&mut game);
    let card = id(&game, Player::Corp, "Seamless Launch");
    choose(&mut game, Player::Corp, Action::PlayCard(card));
    let view = game.observe(Player::Corp);
    assert_eq!(view.corp_credits, 0);
    assert_eq!(view.clicks[0], 2);
    assert_eq!(view.archives[0].printing, Some("30040"));
    assert_eq!(view.decision.unwrap().choices, vec![Action::PassPriority]);
}

#[test]
fn hidden_decks_and_opponent_hands_do_not_change_policy_inputs() {
    for actor in [Player::Corp, Player::Runner] {
        let mut first = EngineeringPosition::economy(42);
        first.active = actor;
        first.shuffle_decks = false;
        let mut second = first.clone();
        for card in &mut second.cards {
            if actor == Player::Runner && matches!(card.zone, Zone::Hq | Zone::Rd) {
                card.kind = CardKind::GovernmentSubsidy;
            }
            if actor == Player::Corp && matches!(card.zone, Zone::Grip | Zone::Stack) {
                card.kind = CardKind::SureGamble;
            }
        }
        let a = Match::engineering_turns(first).unwrap().observe(actor);
        let b = Match::engineering_turns(second).unwrap().observe(actor);
        assert_eq!(a, b);
        assert_eq!(
            netrunner_unfolded::game::baseline_response(&a),
            netrunner_unfolded::game::baseline_response(&b)
        );
    }
}

#[test]
fn legacy_art_launch_cannot_load_an_expanded_turn_save() {
    use netrunner_unfolded::play::{Admission, Play, SaveOptions};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("turns.json");
    let mut turns = Play::engineering(
        EngineeringPosition::economy(42),
        Some(SaveOptions {
            path: path.clone(),
            recover_from: None,
        }),
        Player::Corp,
    );
    turns.admit();
    drop(turns);
    let before = std::fs::read(&path).unwrap();
    let mut legacy = Play::saved(&path);
    legacy.admit();
    assert!(matches!(legacy.admission(), Admission::Refused(_)));
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn saved_turn_counter_boundaries_refuse_without_panicking_or_changing_observation() {
    let game = Match::engineering_turns(EngineeringPosition::economy(42)).unwrap();
    for (pointer, value) in [
        ("/state/corp_credits", serde_json::json!(u32::MAX)),
        ("/state/turn/runner_credits", serde_json::json!(u32::MAX)),
        ("/state/turn/number", serde_json::json!(u64::MAX)),
        ("/state/revision", serde_json::json!(u64::MAX)),
    ] {
        let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
        *checkpoint.pointer_mut(pointer).unwrap() = value;
        assert!(Match::resume(serde_json::from_value(checkpoint).unwrap()).is_err());
    }
    let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
    checkpoint["state"]["turn"]["number"] = serde_json::json!(u64::MAX - 1);
    checkpoint["state"]["turn"]["clicks"] = serde_json::json!([0, 0]);
    checkpoint["state"]["pending"] =
        serde_json::json!({"Paid":{"priority":"Runner","passed":true,"window":"End"}});
    let mut game = Match::resume(serde_json::from_value(checkpoint).unwrap()).unwrap();
    let before = game.observe(Player::Runner);
    let revision = before.decision.as_ref().unwrap().revision;
    assert!(
        game.choose(Player::Runner, revision, Action::PassPriority)
            .is_err()
    );
    assert_eq!(game.observe(Player::Runner), before);
}

#[test]
fn drawing_the_last_rd_card_is_legal_and_a_later_empty_draw_is_still_selectable() {
    let mut position = EngineeringPosition::economy(9);
    position.cards.retain(|c| c.zone != Zone::Rd);
    position
        .cards
        .push(EngineeringCard::new(CardKind::HedgeFund, Zone::Rd));
    let mut game = Match::engineering_turns(position).unwrap();
    pass_window(&mut game);
    assert_eq!(game.observe(Player::Corp).rd_count, 0);
    assert_eq!(game.observe(Player::Corp).match_winner, None);
    pass_window(&mut game);
    choose(&mut game, Player::Corp, Action::Draw);
    assert_eq!(
        game.observe(Player::Corp).match_winner,
        Some(Player::Runner)
    );
}

// CR 5.6.1: the three clicks exist before the mandatory draw, which costs none.
#[test]
fn corp_draws_after_both_players_pass_the_start_scoring_window() {
    let mut game = Match::engineering_turns(EngineeringPosition::economy(42)).unwrap();
    let start = game.observe(Player::Corp);
    assert_eq!(start.clicks, [3, 0]);
    let count = start.hq_count;
    pass_window(&mut game);
    let drawn = game.observe(Player::Corp);
    assert_eq!(drawn.hq_count, count + 1);
    assert_eq!(drawn.clicks, [3, 0]);
    pass_window(&mut game);
    choose(&mut game, Player::Corp, Action::GainCredit);
    assert_eq!(game.observe(Player::Corp).corp_credits, 6);
    assert_eq!(game.observe(Player::Corp).clicks, [2, 0]);
}
