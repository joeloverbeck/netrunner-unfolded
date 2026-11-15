//! Issue #1 / #26; NSG Brân ruling (2021-10-10), CR 26.03 6.2 and 8.5.16b.
//! Every fixture enters the ordinary match transitions and can continue its turn.
use netrunner_unfolded::game::{Action, CardId, Match, Player, RunResult};

fn choose(game: &mut Match, actor: Player, action: Action) {
    let revision = game
        .observe(actor)
        .decision
        .expect("acting player")
        .revision;
    game.choose(actor, revision, action)
        .unwrap_or_else(|e| panic!("{action:?} at {}: {e}", game.observe(actor).phase));
    let bytes = serde_json::to_vec(&game.checkpoint()).unwrap();
    let restored = Match::resume(serde_json::from_slice(&bytes).unwrap()).unwrap();
    for viewer in [Player::Corp, Player::Runner] {
        assert_eq!(game.observe(viewer), restored.observe(viewer));
    }
    *game = restored;
}
fn pass_to(game: &mut Match, wanted: Action) {
    for _ in 0..20 {
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        if game
            .observe(actor)
            .decision
            .unwrap()
            .choices
            .contains(&wanted)
        {
            return;
        }
        choose(game, actor, Action::PassPriority);
    }
    panic!("did not reach {wanted:?}");
}
fn ids(game: &Match) -> (CardId, CardId) {
    let view = game.observe(Player::Corp);
    (
        view.hand[0].instance.unwrap(),
        view.servers[0].ice[0].instance.unwrap(),
    )
}
fn install(game: &mut Match, trash: bool) -> CardId {
    let (ice, bran) = ids(game);
    choose(game, Player::Corp, Action::InstallInward(ice));
    assert_eq!(
        game.observe(Player::Corp).installing.unwrap().instance,
        Some(ice)
    );
    assert!(
        game.observe(Player::Runner)
            .installing
            .unwrap()
            .printing
            .is_none()
    );
    if trash {
        choose(game, Player::Corp, Action::TrashInstalled(bran));
    }
    choose(game, Player::Corp, Action::FinishInstall);
    if trash {
        pass_to(game, Action::Continue);
    }
    ice
}
fn finish_by_passing(game: &mut Match) {
    for _ in 0..24 {
        if game.observe(Player::Corp).run_result.is_some() {
            return;
        }
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        let choices = game.observe(actor).decision.unwrap().choices;
        let action = [Action::PassPriority, Action::Continue, Action::FinishAccess]
            .into_iter()
            .find(|a| choices.contains(a))
            .unwrap();
        choose(game, actor, action);
    }
    panic!("run did not finish");
}
#[test]
fn bran_fixture_uses_ordinary_turns_and_declining_ends_only_the_run() {
    let mut game = Match::prepared_bran_run(3);
    assert_eq!(
        game.observe(Player::Corp).active_player,
        Some(Player::Runner)
    );
    assert_eq!(
        serde_json::to_value(game.checkpoint()).unwrap()["version"],
        2
    );
    choose(&mut game, Player::Corp, Action::DeclineInstall);
    let view = game.observe(Player::Corp);
    assert_eq!(view.run_result, Some(RunResult::EndedByIce));
    assert_eq!(view.corp_credits, 3);
    assert_eq!(view.match_winner, None);
    assert!(
        game.observe(Player::Runner).decision.is_some(),
        "the ordinary turn continues"
    );
}
#[test]
fn retaining_bran_installs_inward_for_free_then_resolves_the_remaining_subroutine() {
    let mut game = Match::prepared_bran_run(3);
    install(&mut game, false);
    let view = game.observe(Player::Corp);
    assert_eq!(view.run_result, Some(RunResult::EndedByIce));
    assert_eq!(view.corp_credits, 3);
    assert_eq!(
        view.servers[0]
            .ice
            .iter()
            .map(|c| c.printing)
            .collect::<Vec<_>>(),
        vec![Some("30072"), Some("30039")]
    );
    assert!(!view.servers[0].ice[0].rezzed);
}
#[test]
fn trashing_bran_preserves_inward_position_and_normal_rez_payment() {
    let mut game = Match::prepared_bran_run(3);
    let ice = install(&mut game, true);
    let movement = game.observe(Player::Corp);
    assert_eq!(movement.archives[0].printing, Some("30039"));
    assert_eq!(movement.corp_credits, 3);
    assert_eq!(movement.run_result, None);
    assert_eq!(movement.servers[0].ice.len(), 1);
    choose(&mut game, Player::Runner, Action::Continue);
    pass_to(&mut game, Action::Rez(ice));
    assert!(game.observe(Player::Corp).phase.starts_with("Approach"));
    choose(&mut game, Player::Corp, Action::Rez(ice));
    assert_eq!(game.observe(Player::Corp).corp_credits, 0);
    assert_eq!(
        game.observe(Player::Runner).servers[0].ice[0].printing,
        Some("30072")
    );
    finish_by_passing(&mut game);
    assert_eq!(
        game.observe(Player::Corp).run_result,
        Some(RunResult::EndedByIce)
    );
}
#[test]
fn unaffordable_rez_refuses_atomically_then_empty_hq_breach_continues_the_match() {
    let mut game = Match::prepared_bran_run(2);
    let ice = install(&mut game, true);
    choose(&mut game, Player::Runner, Action::Continue);
    // Movement paid window and approach window each give both sides priority.
    for _ in 0..3 {
        let actor = [Player::Corp, Player::Runner]
            .into_iter()
            .find(|&p| game.observe(p).decision.is_some())
            .unwrap();
        choose(&mut game, actor, Action::PassPriority);
    }
    let before = serde_json::to_vec(&game.checkpoint()).unwrap();
    let rev = game.observe(Player::Corp).decision.unwrap().revision;
    assert!(game.choose(Player::Corp, rev, Action::Rez(ice)).is_err());
    assert_eq!(serde_json::to_vec(&game.checkpoint()).unwrap(), before);
    finish_by_passing(&mut game);
    let view = game.observe(Player::Runner);
    assert_eq!(view.run_result, Some(RunResult::SuccessfulAccess));
    assert_eq!(view.match_winner, None);
    assert_eq!(view.corp_credits, 2);
    assert!(view.servers[0].ice[0].printing.is_none());
    assert!(!view.history.iter().any(|line| line.contains("Palisade")));
}
#[test]
fn runner_can_jack_out_before_and_after_passing_the_new_unrezzed_ice() {
    for after in [false, true] {
        let mut game = Match::prepared_bran_run(3);
        install(&mut game, true);
        if after {
            choose(&mut game, Player::Runner, Action::Continue);
            pass_to(&mut game, Action::JackOut);
        }
        choose(&mut game, Player::Runner, Action::JackOut);
        assert_eq!(
            game.observe(Player::Runner).run_result,
            Some(RunResult::JackedOut)
        );
    }
}
#[test]
fn stale_wrong_actor_and_unsupported_choices_do_not_change_either_observation() {
    let mut game = Match::prepared_bran_run(3);
    let (ice, bran) = ids(&game);
    choose(&mut game, Player::Corp, Action::InstallInward(ice));
    let before = serde_json::to_vec(&game.checkpoint()).unwrap();
    let revision = game.observe(Player::Corp).decision.unwrap().revision;
    for (actor, rev, action) in [
        (Player::Runner, revision, Action::TrashInstalled(bran)),
        (Player::Corp, revision - 1, Action::FinishInstall),
        (Player::Corp, revision, Action::Rez(ice)),
    ] {
        assert!(game.choose(actor, rev, action).is_err());
        assert_eq!(serde_json::to_vec(&game.checkpoint()).unwrap(), before);
    }
}
#[test]
fn unsupported_legacy_continuations_and_dangling_run_cards_are_refused() {
    let original = serde_json::to_value(Match::prepared_bran_run(3).checkpoint()).unwrap();
    for (pointer, value) in [
        ("/version", serde_json::json!(1)),
        ("/rules", serde_json::json!("unknown")),
        ("/card_revision", serde_json::json!("unknown")),
        ("/state/turn", serde_json::Value::Null),
        ("/state/turn/run/positions/0", serde_json::json!(999)),
    ] {
        let mut invalid = original.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(Match::resume(serde_json::from_value(invalid).unwrap()).is_err());
    }
}

#[test]
fn hidden_replacement_identity_does_not_change_runner_inputs_through_install_and_movement() {
    // Preserve the old paired-face regression at the actual ordinary seam.
    let mut first = Match::prepared_bran_run(3);
    let mut checkpoint = serde_json::to_value(first.checkpoint()).unwrap();
    checkpoint["state"]["cards"][1]["kind"] = serde_json::json!("Diviner");
    let mut other = Match::resume(serde_json::from_value(checkpoint).unwrap()).unwrap();
    let (ice, bran) = ids(&first);
    for action in [
        Action::InstallInward(ice),
        Action::TrashInstalled(bran),
        Action::FinishInstall,
        Action::PassPriority,
        Action::PassPriority,
        Action::Continue,
    ] {
        assert_eq!(first.observe(Player::Runner), other.observe(Player::Runner));
        assert_ne!(first.observe(Player::Corp), other.observe(Player::Corp));
        for game in [&mut first, &mut other] {
            let actor = [Player::Corp, Player::Runner]
                .into_iter()
                .find(|&p| game.observe(p).decision.is_some())
                .unwrap();
            choose(game, actor, action);
        }
    }
    assert_eq!(first.observe(Player::Runner), other.observe(Player::Runner));
}
