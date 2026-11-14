use netrunner_unfolded::game::{Action, EngineeringPosition, Match, Player};

fn choose(game: &mut Match, player: Player, action: Action) {
    let decision = game.observe(player).decision.expect("acting player");
    game.choose(player, decision.revision, action).unwrap();
}

fn pass_window(game: &mut Match) {
    for _ in 0..2 {
        let player = [Player::Corp, Player::Runner].into_iter().find(|&p| game.observe(p).decision.is_some()).unwrap();
        choose(game, player, Action::PassPriority);
    }
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
