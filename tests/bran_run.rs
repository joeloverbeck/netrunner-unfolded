//! Independent expectations: issue #1; NSG Brân ruling (2021-10-10),
//! card-data 6ff880724741ea6ac062acf77e0cf979bb44e67e; CR 26.03.
use netrunner_unfolded::game::{Action, Match, Player, RunResult};

#[test]
fn declining_installation_ends_only_the_run_without_spending_credits() {
    let mut game = Match::prepared_bran_run(3);
    let before = game.observe(Player::Corp);
    let decision = before.decision.unwrap();
    assert!(decision.choices.contains(&Action::DeclineInstall));
    game.choose(Player::Corp, decision.revision, Action::DeclineInstall)
        .unwrap();
    let after = game.observe(Player::Corp);
    assert_eq!(after.run_result, Some(RunResult::EndedByIce));
    assert_eq!(after.corp_credits, 3);
    assert_eq!(after.match_winner, None);
    assert_eq!(after.decision, None);
}

fn choose(game: &mut Match, player: Player, action: Action) {
    let decision = game.observe(player).decision.expect("player decision");
    assert!(decision.choices.contains(&action));
    game.choose(player, decision.revision, action).unwrap();
}

#[test]
fn installing_and_retaining_bran_finishes_installation_then_ends_the_run() {
    let mut game = Match::prepared_bran_run(3);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    assert_eq!(game.observe(Player::Corp).run_result, None);
    choose(&mut game, Player::Corp, Action::RetainBran);
    let view = game.observe(Player::Corp);
    assert_eq!(view.run_result, Some(RunResult::EndedByIce));
    assert_eq!(view.corp_credits, 3);
    assert_eq!(view.hq_count, 0);
    assert_eq!(
        view.ice
            .iter()
            .map(|card| card.title.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("Palisade"), Some("Brân 1.0")]
    );
    assert!(!view.ice[0].rezzed);
}

#[test]
fn replacing_bran_preserves_the_inward_position_and_pays_the_later_rez_cost() {
    let mut game = Match::prepared_bran_run(3);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    choose(&mut game, Player::Corp, Action::TrashBran);
    let movement = game.observe(Player::Corp);
    assert_eq!(movement.run_result, None);
    assert_eq!(movement.corp_credits, 3);
    assert_eq!(movement.ice.len(), 1);
    assert_eq!(movement.archives[0].title.as_deref(), Some("Brân 1.0"));
    choose(&mut game, Player::Runner, Action::Continue);
    assert_eq!(game.observe(Player::Corp).approached_ice, Some(0));
    choose(&mut game, Player::Corp, Action::RezPalisade);
    let ended = game.observe(Player::Corp);
    assert_eq!(ended.corp_credits, 0);
    assert_eq!(ended.run_result, Some(RunResult::EndedByIce));
    assert!(game.observe(Player::Runner).ice[0].rezzed);
    assert_eq!(
        game.observe(Player::Runner).ice[0].title.as_deref(),
        Some("Palisade")
    );
}

#[test]
fn declining_rez_allows_a_successful_breach_of_empty_hq_without_ending_the_match() {
    let mut game = Match::prepared_bran_run(3);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    choose(&mut game, Player::Corp, Action::TrashBran);
    choose(&mut game, Player::Runner, Action::Continue);
    choose(&mut game, Player::Corp, Action::DeclineRez);
    assert_eq!(game.observe(Player::Runner).run_result, None);
    choose(&mut game, Player::Runner, Action::Continue);
    let view = game.observe(Player::Runner);
    assert_eq!(view.run_result, Some(RunResult::SuccessfulEmptyHq));
    assert_eq!(view.match_winner, None);
    assert_eq!(view.corp_credits, 3);
    assert_eq!(view.ice[0].title, None);
}

#[test]
fn runner_can_jack_out_in_each_movement_window() {
    for after_palisade in [false, true] {
        let mut game = Match::prepared_bran_run(3);
        choose(&mut game, Player::Corp, Action::InstallPalisade);
        choose(&mut game, Player::Corp, Action::TrashBran);
        if after_palisade {
            choose(&mut game, Player::Runner, Action::Continue);
            choose(&mut game, Player::Corp, Action::DeclineRez);
        }
        choose(&mut game, Player::Runner, Action::JackOut);
        let view = game.observe(Player::Runner);
        assert_eq!(view.run_result, Some(RunResult::JackedOut));
        assert_eq!(view.corp_credits, 3);
        assert_eq!(view.approached_ice, None);
    }
}

#[test]
fn two_credits_allow_installation_but_not_rez_and_refusal_is_atomic() {
    let mut game = Match::prepared_bran_run(2);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    choose(&mut game, Player::Corp, Action::TrashBran);
    choose(&mut game, Player::Runner, Action::Continue);
    let before = game.observe(Player::Corp);
    let revision = before.decision.as_ref().unwrap().revision;
    assert!(
        !before
            .decision
            .as_ref()
            .unwrap()
            .choices
            .contains(&Action::RezPalisade)
    );
    assert!(
        game.choose(Player::Corp, revision, Action::RezPalisade)
            .is_err()
    );
    assert_eq!(before, game.observe(Player::Corp));
    choose(&mut game, Player::Corp, Action::DeclineRez);
    choose(&mut game, Player::Runner, Action::Continue);
    assert_eq!(
        game.observe(Player::Corp).run_result,
        Some(RunResult::SuccessfulEmptyHq)
    );
}

#[test]
fn both_observations_remain_unchanged_when_actor_revision_or_choice_is_invalid() {
    let mut game = Match::prepared_bran_run(3);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    let before = [game.observe(Player::Corp), game.observe(Player::Runner)];
    let revision = before[0].decision.as_ref().unwrap().revision;
    for (player, rev, action) in [
        (Player::Runner, revision, Action::TrashBran),
        (Player::Corp, revision - 1, Action::TrashBran),
        (Player::Corp, revision, Action::RezPalisade),
    ] {
        assert!(game.choose(player, rev, action).is_err());
        assert_eq!(
            before,
            [game.observe(Player::Corp), game.observe(Player::Runner)]
        );
    }
}

#[test]
fn hidden_faces_inspection_choices_and_history_are_projected_for_the_viewer() {
    let mut game = Match::prepared_bran_run(3);
    assert_eq!(
        game.observe(Player::Corp).hand[0].title.as_deref(),
        Some("Palisade")
    );
    assert_eq!(game.observe(Player::Runner).hand[0].title, None);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    let installing = game.observe(Player::Corp);
    assert_eq!(
        installing.installing.as_ref().unwrap().title.as_deref(),
        Some("Palisade")
    );
    choose(&mut game, Player::Corp, Action::TrashBran);
    let runner = game.observe(Player::Runner);
    assert_eq!(runner.ice[0].printing, None);
    assert_eq!(runner.ice[0].rules_text, None);
    assert!(!format!("{runner:?}").contains("Palisade"));
    choose(&mut game, Player::Runner, Action::Continue);
    assert_eq!(game.observe(Player::Runner).decision, None);
    assert!(game.observe(Player::Corp).ice[0].rules_text.is_some());
}

#[test]
fn fixed_runner_uses_only_its_legal_observation_and_history_explains_the_completed_run() {
    use netrunner_unfolded::game::fixed_runner_response;
    let mut game = Match::prepared_bran_run(3);
    assert_eq!(fixed_runner_response(&game.observe(Player::Runner)), None);
    choose(&mut game, Player::Corp, Action::InstallPalisade);
    choose(&mut game, Player::Corp, Action::TrashBran);
    assert_eq!(fixed_runner_response(&game.observe(Player::Corp)), None);
    let runner = game.observe(Player::Runner);
    let action = fixed_runner_response(&runner).unwrap();
    game.choose(Player::Runner, runner.decision.unwrap().revision, action)
        .unwrap();
    choose(&mut game, Player::Corp, Action::DeclineRez);
    let runner = game.observe(Player::Runner);
    game.choose(
        Player::Runner,
        runner.decision.as_ref().unwrap().revision,
        fixed_runner_response(&runner).unwrap(),
    )
    .unwrap();
    let runner = game.observe(Player::Runner);
    assert!(runner.history.iter().any(|line| line.contains("empty HQ")));
    assert!(!runner.history.iter().any(|line| line.contains("Palisade")));
    assert!(
        game.observe(Player::Corp)
            .history
            .iter()
            .any(|line| line.contains("Palisade"))
    );
}
