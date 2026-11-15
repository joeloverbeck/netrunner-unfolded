//! #20: source-derived expectations from CR 26.03 (5.2, 8.4, 9.6.5,
//! 10.1.1), NSG card text 6ff8807, and the verified event last-click rulings.
use netrunner_unfolded::game::{
    Action, CardId, CardKind, EngineeringCard, EngineeringPosition, Match, Player, Server, Zone,
    baseline_response,
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
        "{action:?} unavailable: {:?}",
        d.choices
    );
    game.choose(d.actor, d.revision, action).unwrap();
    let resumed = Match::resume(game.checkpoint()).unwrap();
    for player in [Player::Corp, Player::Runner] {
        assert_eq!(game.observe(player), resumed.observe(player));
    }
    *game = resumed;
}
fn action_phase(game: &mut Match) {
    for _ in 0..12 {
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
            baseline_response(&view).unwrap()
        };
        choose(game, action);
    }
    panic!("Turn did not end");
}
fn position(cards: &[(CardKind, Zone)], stack: usize) -> EngineeringPosition {
    let mut p = EngineeringPosition::economy(20);
    p.active = Player::Runner;
    p.runner_credits = 10;
    p.shuffle_decks = false;
    p.cards = cards
        .iter()
        .map(|&(kind, zone)| {
            let mut card = EngineeringCard::new(kind, zone);
            if zone == Zone::Rig {
                card.installed_turn = Some(1);
            }
            card
        })
        .collect();
    p.cards
        .extend((0..stack).map(|_| EngineeringCard::new(CardKind::SureGamble, Zone::Stack)));
    p.cards
        .extend((0..6).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Rd)));
    p
}
fn id(game: &Match, title: &str) -> CardId {
    let view = game.observe(Player::Runner);
    view.hand
        .iter()
        .find(|c| c.title.as_deref() == Some(title))
        .unwrap()
        .instance
        .unwrap()
}
fn install(game: &mut Match, title: &str) -> CardId {
    action_phase(game);
    let card = id(game, title);
    choose(game, Action::InstallCard(card));
    choose(game, Action::FinishInstall);
    card
}

#[test]
fn first_basic_draw_before_installation_survives_restart_and_resets_on_real_turn() {
    let mut game =
        Match::engineering_turns(position(&[(CardKind::VerbalPlasticity, Zone::Grip)], 8)).unwrap();
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    assert_eq!(game.observe(Player::Runner).hand.len(), 2);
    install(&mut game, "Verbal Plasticity");
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    let view = game.observe(Player::Runner);
    assert_eq!(
        view.hand.len(),
        2,
        "earlier basic draw used this turn's first occurrence"
    );
    assert_eq!(view.runner_credits, 7);
    next_turn(&mut game);
    next_turn(&mut game);
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    assert_eq!(
        game.observe(Player::Runner).hand.len(),
        4,
        "new Runner turn draws two"
    );
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    assert_eq!(
        game.observe(Player::Runner).hand.len(),
        5,
        "second basic draw draws one"
    );
}

#[test]
fn creative_commission_pays_one_click_and_credit_then_loses_only_an_available_click() {
    for spent in [0, 2, 3] {
        let mut p = position(&[(CardKind::CreativeCommission, Zone::Grip)], 0);
        p.runner_credits = 1;
        let mut game = Match::engineering_turns(p).unwrap();
        for _ in 0..spent {
            action_phase(&mut game);
            choose(&mut game, Action::GainCredit);
        }
        action_phase(&mut game);
        let event = id(&game, "Creative Commission");
        choose(&mut game, Action::PlayCard(event));
        let view = game.observe(Player::Runner);
        assert_eq!(view.runner_credits, 5 + spent);
        assert_eq!(view.clicks[1], if spent == 0 { 2 } else { 0 });
        assert_eq!(view.heap[0].instance, Some(event));
    }
    let mut p = position(&[(CardKind::CreativeCommission, Zone::Grip)], 0);
    p.runner_credits = 0;
    let mut game = Match::engineering_turns(p).unwrap();
    action_phase(&mut game);
    let event = id(&game, "Creative Commission");
    let before = game.observe(Player::Runner);
    let decision = before.decision.as_ref().unwrap();
    assert!(!decision.choices.contains(&Action::PlayCard(event)));
    assert!(
        game.choose(Player::Runner, decision.revision, Action::PlayCard(event))
            .is_err()
    );
    assert_eq!(before, game.observe(Player::Runner));
}

#[test]
fn vrcation_is_event_draw_and_last_click_play_keeps_the_real_discard_decision() {
    let mut game = Match::engineering_turns(position(
        &[
            (CardKind::VerbalPlasticity, Zone::Rig),
            (CardKind::VRcation, Zone::Grip),
        ],
        9,
    ))
    .unwrap();
    action_phase(&mut game);
    let event = id(&game, "VRcation");
    choose(&mut game, Action::PlayCard(event));
    let view = game.observe(Player::Runner);
    assert_eq!(
        view.hand.len(),
        4,
        "Verbal Plasticity does not multiply event draws"
    );
    assert_eq!(view.clicks[1], 2);
    assert_eq!(view.runner_credits, 9);
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    assert_eq!(
        game.observe(Player::Runner).hand.len(),
        6,
        "event draw did not consume first basic draw"
    );

    let mut game = Match::engineering_turns(position(
        &[
            (CardKind::VerbalPlasticity, Zone::Rig),
            (CardKind::VRcation, Zone::Grip),
            (CardKind::Jailbreak, Zone::Grip),
            (CardKind::Jailbreak, Zone::Grip),
            (CardKind::Jailbreak, Zone::Grip),
        ],
        5,
    ))
    .unwrap();
    for _ in 0..3 {
        action_phase(&mut game);
        choose(&mut game, Action::GainCredit);
    }
    action_phase(&mut game);
    let event = id(&game, "VRcation");
    choose(&mut game, Action::PlayCard(event));
    let view = game.observe(Player::Runner);
    assert_eq!(view.hand.len(), 7);
    assert_eq!(view.clicks[1], 0);
    assert_eq!(view.runner_credits, 12);
    choose(&mut game, Action::PassPriority);
    choose(&mut game, Action::PassPriority);
    let view = game.observe(Player::Runner);
    assert_eq!(view.phase, "Choose cards to discard simultaneously");
    assert!(
        !view
            .decision
            .as_ref()
            .unwrap()
            .choices
            .contains(&Action::ConfirmDiscard)
    );
    for card in view.hand.iter().take(2) {
        choose(&mut game, Action::ToggleDiscard(card.instance.unwrap()));
    }
    choose(&mut game, Action::ConfirmDiscard);
    assert_eq!(game.observe(Player::Runner).hand.len(), 5);
    assert_eq!(game.observe(Player::Runner).heap.len(), 3);
}

#[test]
fn replacing_unique_verbal_plasticity_trashes_old_copy_without_resetting_first_draw() {
    let mut game = Match::engineering_turns(position(
        &[
            (CardKind::VerbalPlasticity, Zone::Rig),
            (CardKind::VerbalPlasticity, Zone::Grip),
        ],
        5,
    ))
    .unwrap();
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    let new = install(&mut game, "Verbal Plasticity");
    let view = game.observe(Player::Runner);
    assert_eq!(view.rig.len(), 1);
    assert_eq!(view.rig[0].instance, Some(new));
    assert_eq!(view.heap.len(), 1);
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    assert_eq!(game.observe(Player::Runner).hand.len(), 3);
}

#[test]
fn lawful_labels_explain_event_costs_and_current_first_basic_draw() {
    let mut game = Match::engineering_turns(position(
        &[
            (CardKind::CreativeCommission, Zone::Grip),
            (CardKind::VRcation, Zone::Grip),
            (CardKind::VerbalPlasticity, Zone::Rig),
        ],
        5,
    ))
    .unwrap();
    action_phase(&mut game);
    let view = game.observe(Player::Runner);
    for title in ["Creative Commission", "VRcation"] {
        let label = Action::PlayCard(id(&game, title)).description(&view);
        assert!(label.contains("1 click + 1 credit"), "{label}");
        assert!(label.contains("then lose a click if left"), "{label}");
    }
    assert!(Action::Draw.description(&view).contains("Draw 2"));
    choose(&mut game, Action::Draw);
    assert!(
        Action::Draw
            .description(&game.observe(Player::Runner))
            .contains("Draw 1")
    );
    // Corp may know a basic action occurred but gains no private card identity.
    let corp = game.observe(Player::Corp);
    assert!(
        corp.hand
            .iter()
            .all(|c| c.printing.is_none() || c.printing == Some("30075"))
    );
    assert!(!corp.history.iter().any(|line| line.contains("Sure Gamble")));
}

#[test]
fn baseline_uses_economy_draw_event_and_installed_first_draw_from_lawful_information() {
    for (cards, expected) in [
        (
            vec![(CardKind::CreativeCommission, Zone::Grip)],
            "Creative Commission",
        ),
        (vec![(CardKind::VRcation, Zone::Grip)], "VRcation"),
        (
            vec![(CardKind::VerbalPlasticity, Zone::Grip)],
            "Verbal Plasticity",
        ),
    ] {
        let mut game = Match::engineering_turns(position(&cards, 5)).unwrap();
        action_phase(&mut game);
        let view = game.observe(Player::Runner);
        let action = baseline_response(&view).unwrap();
        let card = id(&game, expected);
        assert_eq!(
            action,
            if expected == "Verbal Plasticity" {
                Action::InstallCard(card)
            } else {
                Action::PlayCard(card)
            }
        );
    }
    let mut game =
        Match::engineering_turns(position(&[(CardKind::VerbalPlasticity, Zone::Rig)], 5)).unwrap();
    action_phase(&mut game);
    let view = game.observe(Player::Runner);
    assert_eq!(baseline_response(&view), Some(Action::Draw));
    let mut other = position(&[(CardKind::VerbalPlasticity, Zone::Rig)], 5);
    for card in &mut other.cards {
        if card.zone == Zone::Stack {
            card.kind = CardKind::Unity;
        }
    }
    let mut other = Match::engineering_turns(other).unwrap();
    action_phase(&mut other);
    assert_eq!(
        view,
        other.observe(Player::Runner),
        "hidden stack identities are absent from policy input"
    );
    assert_eq!(
        baseline_response(&view),
        baseline_response(&other.observe(Player::Runner))
    );
}

#[test]
fn corp_baseline_does_not_treat_runner_verbal_plasticity_as_its_draw_ability() {
    for zone in [Zone::Rig, Zone::Heap] {
        let mut p = position(&[(CardKind::VerbalPlasticity, zone)], 5);
        p.active = Player::Corp;
        p.corp_credits = 0;
        let mut game = Match::engineering_turns(p).unwrap();
        action_phase(&mut game);
        let view = game.observe(Player::Corp);
        assert_eq!(view.hand.len(), 1, "Corp took its mandatory draw");
        assert_eq!(baseline_response(&view), Some(Action::GainCredit));
        choose(&mut game, baseline_response(&view).unwrap());
        assert_eq!(game.observe(Player::Corp).corp_credits, 1);
        assert_eq!(game.observe(Player::Corp).hand.len(), 1);
    }
}

#[test]
fn short_and_empty_runner_stack_resolve_available_draws_without_a_deckout() {
    for stack in [0, 1, 3] {
        let mut game = Match::engineering_turns(position(
            &[
                (CardKind::VRcation, Zone::Grip),
                (CardKind::VerbalPlasticity, Zone::Rig),
            ],
            stack,
        ))
        .unwrap();
        action_phase(&mut game);
        let event = id(&game, "VRcation");
        choose(&mut game, Action::PlayCard(event));
        let view = game.observe(Player::Runner);
        assert_eq!(view.hand.len(), stack);
        assert_eq!(view.stack_count, 0);
        assert_eq!(view.clicks[1], 2);
        assert!(view.match_winner.is_none());
        action_phase(&mut game);
        choose(&mut game, Action::Draw);
        let view = game.observe(Player::Runner);
        assert_eq!(view.hand.len(), stack);
        assert_eq!(view.clicks[1], 1);
        assert!(view.match_winner.is_none());
        assert!(
            view.history
                .iter()
                .any(|line| line.contains("stack is empty")),
            "the failed draw is explained"
        );
    }
    for stack in [0, 1] {
        let mut game =
            Match::engineering_turns(position(&[(CardKind::VerbalPlasticity, Zone::Rig)], stack))
                .unwrap();
        action_phase(&mut game);
        choose(&mut game, Action::Draw);
        assert_eq!(game.observe(Player::Runner).hand.len(), stack);
        assert!(game.observe(Player::Runner).match_winner.is_none());
    }
}

#[test]
fn vrcation_and_verbal_plasticity_refuse_unpayable_costs_without_changing_state() {
    for (kind, title, credits, install) in [
        (CardKind::VRcation, "VRcation", 0, false),
        (CardKind::VerbalPlasticity, "Verbal Plasticity", 2, true),
    ] {
        let mut p = position(&[(kind, Zone::Grip)], 5);
        p.runner_credits = credits;
        let mut game = Match::engineering_turns(p).unwrap();
        action_phase(&mut game);
        let card = id(&game, title);
        let action = if install {
            Action::InstallCard(card)
        } else {
            Action::PlayCard(card)
        };
        let before = game.observe(Player::Runner);
        let decision = before.decision.as_ref().unwrap();
        assert!(!decision.choices.contains(&action));
        assert!(
            game.choose(Player::Runner, decision.revision, action)
                .is_err()
        );
        assert_eq!(before, game.observe(Player::Runner));
    }
}

#[test]
fn jailbreak_success_draw_does_not_consume_or_receive_the_basic_draw_replacement() {
    let mut game = Match::engineering_turns(position(
        &[
            (CardKind::VerbalPlasticity, Zone::Rig),
            (CardKind::Jailbreak, Zone::Grip),
        ],
        5,
    ))
    .unwrap();
    action_phase(&mut game);
    let event = id(&game, "Jailbreak");
    choose(&mut game, Action::PlayCard(event));
    choose(&mut game, Action::RunServer(Server::Hq));
    for _ in 0..30 {
        let view = game.observe(Player::Runner);
        if view.run.is_none() && view.heap.iter().any(|c| c.instance == Some(event)) {
            break;
        }
        let acting = [Player::Corp, Player::Runner]
            .into_iter()
            .map(|p| game.observe(p))
            .find(|v| v.decision.is_some())
            .unwrap();
        choose(&mut game, baseline_response(&acting).unwrap());
    }
    let view = game.observe(Player::Runner);
    assert!(view.run.is_none());
    assert_eq!(view.hand.len(), 1);
    assert!(!view.runner_basic_draw_taken);
    action_phase(&mut game);
    choose(&mut game, Action::Draw);
    assert_eq!(game.observe(Player::Runner).hand.len(), 3);
}
