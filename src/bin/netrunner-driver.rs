//! Asset-independent engineering probe of the production match boundary.
use netrunner_unfolded::game::{Action, Match, Player, fixed_runner_response};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--help") {
        println!(
            "Engineering check: netrunner-driver [decline|keep|rez|unrezzed|jack-out] [2|3]\nNative play uses netrunner-tabletop and requires prepared artwork."
        );
        return Ok(());
    }
    let branch = args.first().map(String::as_str).unwrap_or("rez");
    if !["decline", "keep", "rez", "unrezzed", "jack-out"].contains(&branch) || args.len() > 2 {
        return Err("Use --help for the finite engineering cases.".into());
    }
    let credits = match args.get(1).map(String::as_str) {
        None | Some("3") => 3,
        Some("2") => 2,
        _ => return Err("The engineering cases use 2 or 3 credits.".into()),
    };
    let mut game = Match::prepared_bran_run(credits);
    loop {
        let corp = game.observe(Player::Corp);
        if let Some(result) = corp.run_result {
            for event in corp.history {
                println!("{event}");
            }
            println!(
                "Run result: {result:?}; match winner: {:?}",
                corp.match_winner
            );
            return Ok(());
        }
        let runner = game.observe(Player::Runner);
        let (view, action) = if let Some(decision) = &corp.decision {
            let candidates = match branch {
                "decline" => [
                    Action::DeclineInstall,
                    Action::RetainBran,
                    Action::DeclineRez,
                ],
                "keep" => [
                    Action::InstallPalisade,
                    Action::RetainBran,
                    Action::DeclineRez,
                ],
                "rez" => [
                    Action::InstallPalisade,
                    Action::TrashBran,
                    Action::RezPalisade,
                ],
                _ => [
                    Action::InstallPalisade,
                    Action::TrashBran,
                    Action::DeclineRez,
                ],
            };
            let action = candidates
                .into_iter()
                .find(|action| decision.choices.contains(action))
                .or_else(|| {
                    decision
                        .choices
                        .contains(&Action::DeclineRez)
                        .then_some(Action::DeclineRez)
                })
                .ok_or("The requested probe has no legal Corp choice.")?;
            (corp, action)
        } else {
            let action = if branch == "jack-out" {
                Some(Action::JackOut)
            } else {
                fixed_runner_response(&runner)
            };
            (runner, action.ok_or("No legal Runner response.")?)
        };
        let decision = view.decision.ok_or("No current decision.")?;
        game.choose(decision.actor, decision.revision, action)?;
    }
}
