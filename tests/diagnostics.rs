use netrunner_unfolded::{
    game::{Action, Match, Player},
    policies::Policy,
};

#[test]
fn frozen_roles_only_receive_their_own_current_decision() {
    let game = Match::introductory(42);
    let corp = game.observe(Player::Corp);
    assert_eq!(Policy::CorpV0.choose(&corp), Some(Action::KeepHand));
    assert_eq!(Policy::RunnerV0.choose(&corp), None);
    assert_eq!(Policy::RunnerV0.choose(&game.observe(Player::Runner)), None);
    assert_ne!(Policy::CorpV0.identity(), Policy::RunnerV0.identity());
    // Version custody, not a tactical oracle: v0 must remain available byte-for-byte.
    assert_eq!(
        Policy::CorpV0.identity(),
        "CorpV0:sha256:9d63853780825ae3ccf4bcda4e3872807b009847064a8e7ac3d76de05d032a71"
    );
}

#[test]
fn recorded_match_prefix_reproduces_and_keeps_role_records_separate() {
    use netrunner_unfolded::diagnostics::{Limits, record_introductory, reproduce};
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("prefix");
    let summary = record_introductory(
        42,
        &output,
        Limits {
            decisions: 6,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(summary.decisions, 6);
    assert_eq!(
        summary.ending,
        netrunner_unfolded::diagnostics::Ending::Censored("decision limit".into())
    );
    assert_eq!(
        reproduce(&output.join("private.jsonl")).unwrap(),
        summary.final_state
    );
    let corp = std::fs::read_to_string(output.join("corp.jsonl")).unwrap();
    let runner = std::fs::read_to_string(output.join("runner.jsonl")).unwrap();
    for (text, role) in [(corp, "Corp"), (runner, "Runner")] {
        assert!(!text.contains("random_state"));
        assert!(!text.contains("checkpoint"));
        for line in text.lines() {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(row["observation"]["decision"]["actor"], role);
            assert!(row.get("terminal_reason").is_none());
        }
    }
}

fn choose(game: &mut Match, action: Action) {
    let d = [Player::Corp, Player::Runner]
        .into_iter()
        .find_map(|p| game.observe(p).decision)
        .unwrap();
    game.choose(d.actor, d.revision, action).unwrap();
}

#[test]
fn hub_diagnostic_uses_known_deck_count_and_leaves_other_choices_unassessed() {
    use netrunner_unfolded::{
        diagnostics::{Assessment, assess},
        game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone},
    };
    let mut p = EngineeringPosition::economy(42);
    p.cards = vec![EngineeringCard {
        kind: CardKind::SuperconductingHub,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 3,
        installed_turn: Some(0),
        rezzed: false,
    }];
    let mut game = Match::engineering_turns(p).unwrap();
    let id = game.observe(Player::Corp).servers[3].root[0]
        .instance
        .unwrap();
    choose(&mut game, Action::Score(id));
    let view = game.observe(Player::Corp);
    assert_eq!(
        assess(&view, Action::DrawAgendaCards),
        Assessment::Checked {
            case: "hub_deck_exhaustion",
            mistake: true
        }
    );
    assert_eq!(
        assess(&view, Action::DeclineAgendaAbility),
        Assessment::Checked {
            case: "hub_deck_exhaustion",
            mistake: false
        }
    );
    choose(&mut game, Action::DeclineAgendaAbility);
    assert!(game.observe(Player::Corp).match_winner.is_none());
    assert_eq!(
        assess(
            &Match::introductory(42).observe(Player::Corp),
            Action::KeepHand
        ),
        Assessment::Unassessed
    );
}

#[test]
fn exposed_karuna_lethal_and_immediate_winning_score_have_independent_answers() {
    use netrunner_unfolded::{
        diagnostics::{Assessment, assess},
        game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone},
    };
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.cards = vec![EngineeringCard::new(CardKind::Jailbreak, Zone::Grip); 3];
    p.cards.push(EngineeringCard {
        kind: CardKind::Karuna,
        zone: Zone::Protecting(Server::Hq),
        advancement: 0,
        installed_turn: Some(1),
        rezzed: true,
    });
    let mut game = Match::engineering_turns(p).unwrap();
    for _ in 0..20 {
        if game.observe(Player::Runner).phase == "Action phase" {
            break;
        }
        choose(&mut game, Action::PassPriority);
    }
    choose(&mut game, Action::RunServer(Server::Hq));
    for _ in 0..30 {
        if game.observe(Player::Runner).phase.starts_with("Karunā") {
            break;
        }
        choose(&mut game, Action::PassPriority);
    }
    let view = game.observe(Player::Runner);
    assert_eq!(view.hand.len(), 1); // Three cards minus the first two damage.
    assert_eq!(
        assess(&view, Action::Continue),
        Assessment::Checked {
            case: "karuna_exposed_lethal",
            mistake: true
        }
    );
    assert_eq!(
        assess(&view, Action::JackOut),
        Assessment::Checked {
            case: "karuna_exposed_lethal",
            mistake: false
        }
    );
    choose(&mut game, Action::Continue);
    assert_eq!(
        game.observe(Player::Runner).match_winner,
        Some(Player::Corp)
    );

    let mut p = EngineeringPosition::economy(42);
    p.agenda_points = [4, 0];
    p.cards = vec![EngineeringCard {
        kind: CardKind::OffworldOffice,
        zone: Zone::Root(Server::Remote(1)),
        advancement: 4,
        installed_turn: Some(1),
        rezzed: false,
    }];
    let mut game = Match::engineering_turns(p).unwrap();
    let view = game.observe(Player::Corp);
    let id = view.servers[3].root[0].instance.unwrap();
    assert_eq!(
        assess(&view, Action::PassPriority),
        Assessment::Checked {
            case: "immediate_winning_agenda",
            mistake: true
        }
    );
    assert_eq!(
        assess(&view, Action::Score(id)),
        Assessment::Checked {
            case: "immediate_winning_agenda",
            mistake: false
        }
    );
    choose(&mut game, Action::Score(id));
    assert_eq!(game.observe(Player::Corp).match_winner, Some(Player::Corp));
}

#[test]
fn both_role_policies_ignore_hidden_permutations_through_lawful_prefixes() {
    for (viewer, policy, hidden_hand, hidden_deck, own_hand, own_deck) in [
        (Player::Corp, Policy::CorpV0, "Grip", "Stack", "Hq", "Rd"),
        (Player::Corp, Policy::CorpV1, "Grip", "Stack", "Hq", "Rd"),
        (
            Player::Runner,
            Policy::RunnerV1,
            "Hq",
            "Rd",
            "Grip",
            "Stack",
        ),
        (
            Player::Runner,
            Policy::RunnerV0,
            "Hq",
            "Rd",
            "Grip",
            "Stack",
        ),
    ] {
        let mut original = Match::introductory(42);
        let swap = |game: &Match, hand: &str, deck: &str| {
            let mut checkpoint = serde_json::to_value(game.checkpoint()).unwrap();
            let cards = checkpoint["state"]["cards"].as_array_mut().unwrap();
            let first = cards.iter().position(|c| c["zone"] == hand).unwrap();
            let second = cards
                .iter()
                .position(|c| c["zone"] == deck && c["kind"] != cards[first]["kind"])
                .unwrap();
            let kind = cards[first]["kind"].clone();
            cards[first]["kind"] = cards[second]["kind"].clone();
            cards[second]["kind"] = kind;
            Match::resume(serde_json::from_value(checkpoint).unwrap()).unwrap()
        };
        let mut hidden = swap(&original, hidden_hand, hidden_deck);
        // Positive control: a legitimately visible own card changes the input.
        assert_ne!(
            original.observe(viewer),
            swap(&original, own_hand, own_deck).observe(viewer)
        );
        for _ in 0..12 {
            let a = original.observe(viewer);
            let b = hidden.observe(viewer);
            assert_eq!(a, b);
            assert_eq!(policy.choose(&a), policy.choose(&b));
            let d = [Player::Corp, Player::Runner]
                .into_iter()
                .find_map(|p| original.observe(p).decision)
                .unwrap();
            let action = [Action::KeepHand, Action::PassPriority, Action::GainCredit]
                .into_iter()
                .find(|a| d.choices.contains(a))
                .unwrap();
            choose(&mut original, action);
            choose(&mut hidden, action);
        }
    }
}

#[test]
fn source_derived_suspended_hq_run_has_independent_arithmetic_and_reproduces() {
    use netrunner_unfolded::diagnostics::{Ending, record_source_case, reproduce};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source-case");
    let (summary, view) = record_source_case(&path).unwrap();
    assert_eq!(
        (view.corp_credits, view.runner_credits, view.clicks[1]),
        (0, 4, 3)
    );
    assert_eq!(view.run.as_ref().unwrap().hosted_credits, 3);
    let choices = &view.decision.as_ref().unwrap().choices;
    assert!(choices.contains(&Action::PayManegarmCredits));
    assert!(choices.contains(&Action::SpendManegarmClicks));
    assert!(choices.contains(&Action::RefuseManegarm));
    assert_eq!(
        summary.ending,
        Ending::Censored("selected source-case suspension at Manegarm".into())
    );
    assert_eq!(
        reproduce(&path.join("private.jsonl")).unwrap(),
        summary.final_state
    );
}

#[test]
fn reproduction_refuses_changed_identity_truncation_and_fabricated_terminal() {
    use netrunner_unfolded::diagnostics::{Limits, record_introductory, reproduce};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prefix");
    record_introductory(
        42,
        &path,
        Limits {
            decisions: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let original = std::fs::read_to_string(path.join("private.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = original
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    for mutation in 0..6 {
        let mut rows = rows.clone();
        match mutation {
            0 => rows[0]["identity"]["executable"] = "incorrect".into(),
            1 => {
                rows.pop();
            }
            2 => {
                rows.last_mut().unwrap()["summary"]["ending"] =
                    serde_json::json!({"Terminal":{"winner":"Corp","reason":"fabricated"}})
            }
            3 => rows[0]["policies"][0] = "different policy".into(),
            4 => rows[0]["basis"]["Introductory"]["seed"] = 89.into(),
            _ => rows[1]["action"] = "Draw".into(),
        }
        let file = dir.path().join(format!("altered-{mutation}.jsonl"));
        std::fs::write(
            &file,
            rows.iter().map(|r| format!("{r}\n")).collect::<String>(),
        )
        .unwrap();
        assert!(reproduce(&file).is_err(), "mutation {mutation} must refuse");
    }
    assert!(record_introductory(42, &path, Limits::default()).is_err());
    assert_eq!(
        std::fs::read_to_string(path.join("private.jsonl")).unwrap(),
        original
    );
}

#[test]
fn paired_comparison_retains_both_censored_arms_and_reserves_holdouts() {
    use netrunner_unfolded::diagnostics::{ComparisonPlan, Ending, run_comparison};
    let dir = tempfile::tempdir().unwrap();
    let mut plan = ComparisonPlan::pilot();
    plan.blocks.truncate(1);
    plan.limits.decisions = 2;
    let report = run_comparison(&plan, &dir.path().join("paired")).unwrap();
    assert_eq!(report.games.len(), 2);
    for game in &report.games {
        let summary = game.summary.as_ref().unwrap();
        assert_eq!(summary.ending, Ending::Censored("decision limit".into()));
        assert_eq!(summary.decisions, 2);
    }
    assert_eq!(
        report.games[0].summary.as_ref().unwrap().final_state,
        report.games[1].summary.as_ref().unwrap().final_state
    );
    plan.blocks[0].seed = plan.reserved_seeds[0];
    assert!(run_comparison(&plan, &dir.path().join("invalid")).is_err());
}

#[test]
fn corp_candidate_comparison_reproduces_through_its_actual_policy_identity() {
    use netrunner_unfolded::diagnostics::{ComparisonPlan, reproduce, run_comparison};
    let dir = tempfile::tempdir().unwrap();
    let mut plan = ComparisonPlan::pilot();
    plan.blocks.truncate(1);
    plan.blocks[0].candidate = Policy::CorpV1;
    plan.limits.decisions = 12;
    let output = dir.path().join("candidate");
    let report = run_comparison(&plan, &output).unwrap();
    assert_eq!(
        reproduce(&output.join("0-candidate/private.jsonl")).unwrap(),
        report.games[1].summary.as_ref().unwrap().final_state
    );
    let lawful = std::fs::read_to_string(output.join("0-candidate/corp.jsonl")).unwrap();
    assert!(lawful.contains(&Policy::CorpV1.identity()));
    assert!(!lawful.contains("checkpoint"));
}

#[test]
fn wall_and_storage_caps_are_censored_results_not_netrunner_draws() {
    use netrunner_unfolded::diagnostics::{Ending, Limits, record_introductory, reproduce};
    let dir = tempfile::tempdir().unwrap();
    for (name, limits, reason) in [
        (
            "time",
            Limits {
                milliseconds: 0,
                ..Limits::default()
            },
            "wall time limit",
        ),
        (
            "bytes",
            Limits {
                bytes: 65536,
                ..Limits::default()
            },
            "record byte limit",
        ),
    ] {
        let path = dir.path().join(name);
        let summary = record_introductory(42, &path, limits).unwrap();
        assert_eq!(summary.ending, Ending::Censored(reason.into()));
        assert!(summary.record_bytes <= limits.bytes);
        assert!(summary.decisions < 4000);
        let actual_bytes: u64 = std::fs::read_dir(&path)
            .unwrap()
            .map(|f| f.unwrap().metadata().unwrap().len())
            .sum();
        assert_eq!(summary.record_bytes, actual_bytes);
        assert_eq!(
            reproduce(&path.join("private.jsonl")).unwrap(),
            summary.final_state
        );
    }
}

#[test]
fn winning_steal_is_assessed_only_after_the_agenda_is_lawfully_accessed() {
    use netrunner_unfolded::{
        diagnostics::{Assessment, assess},
        game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone},
    };
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.agenda_points = [0, 3];
    p.cards = vec![EngineeringCard::new(CardKind::SendAMessage, Zone::Rd)];
    let mut game = Match::engineering_turns(p).unwrap();
    for _ in 0..30 {
        let view = game.observe(Player::Runner);
        if view.phase == "Action phase" {
            break;
        }
        choose(&mut game, Action::PassPriority);
    }
    choose(&mut game, Action::RunServer(Server::Rd));
    for _ in 0..30 {
        let view = game.observe(Player::Runner);
        let d = [Player::Runner, Player::Corp]
            .into_iter()
            .find_map(|p| game.observe(p).decision)
            .unwrap();
        if d.choices.contains(&Action::AccessRd) {
            assert_eq!(assess(&view, Action::AccessRd), Assessment::Unassessed);
            choose(&mut game, Action::AccessRd);
            break;
        }
        choose(
            &mut game,
            if d.choices.contains(&Action::Continue) {
                Action::Continue
            } else {
                Action::PassPriority
            },
        );
    }
    let view = game.observe(Player::Runner);
    assert_eq!(
        assess(&view, Action::StealAgenda),
        Assessment::Checked {
            case: "immediate_winning_agenda",
            mistake: false
        }
    );
    choose(&mut game, Action::StealAgenda);
    assert_eq!(
        game.observe(Player::Runner).match_winner,
        Some(Player::Runner)
    );
}

#[test]
fn resolved_karuna_damage_is_not_misreported_as_future_lethal_at_movement() {
    use netrunner_unfolded::{
        diagnostics::{Assessment, assess},
        game::{CardKind, EngineeringCard, EngineeringPosition, Server, Zone},
    };
    let mut p = EngineeringPosition::economy(42);
    p.active = Player::Runner;
    p.cards = vec![EngineeringCard::new(CardKind::Jailbreak, Zone::Grip); 5];
    p.cards.push(EngineeringCard {
        kind: CardKind::Karuna,
        zone: Zone::Protecting(Server::Hq),
        advancement: 0,
        installed_turn: Some(1),
        rezzed: true,
    });
    let mut g = Match::engineering_turns(p).unwrap();
    for _ in 0..12 {
        if g.observe(Player::Runner).phase == "Action phase" {
            break;
        }
        choose(&mut g, Action::PassPriority);
    }
    choose(&mut g, Action::RunServer(Server::Hq));
    for _ in 0..35 {
        let view = g.observe(Player::Runner);
        if view
            .decision
            .as_ref()
            .is_some_and(|d| d.choices.contains(&Action::Continue))
        {
            if !view.run.as_ref().unwrap().karuna_continuation {
                assert_eq!(view.hand.len(), 1);
                assert_eq!(assess(&view, Action::Continue), Assessment::Unassessed);
                return;
            }
            choose(&mut g, Action::Continue);
        } else {
            choose(&mut g, Action::PassPriority);
        }
    }
    panic!("no movement after four resolved damage");
}
