//! Asset-independent engineering probe of the production match boundary.
use netrunner_unfolded::game::{Action, Player, fixed_runner_response};

use netrunner_unfolded::play::{Admission, Play, SaveOptions};

fn main() -> Result<(), String> {
    let mut args = Vec::new();
    let mut input = std::env::args().skip(1);
    let mut save = None;
    let mut recovery = None;
    let mut limit = usize::MAX;
    let mut viewer = Player::Corp;
    let mut json = false;
    while let Some(arg) = input.next() {
        match arg.as_str() {
            "--save" => save = Some(input.next().ok_or("Missing save path")?.into()),
            "--recover-from" => {
                recovery = Some(input.next().ok_or("Missing recovery source")?.into())
            }
            "--stop-after" => {
                limit = input
                    .next()
                    .ok_or("Missing decision count")?
                    .parse()
                    .map_err(|_| "Invalid decision count")?;
                json = true;
            }
            "--view" => {
                viewer = match input.next().as_deref() {
                    Some("corp") => Player::Corp,
                    Some("runner") => Player::Runner,
                    _ => return Err("Use --view corp or runner".into()),
                };
                json = true;
            }
            _ => args.push(arg),
        }
    }
    if recovery.is_some() && save.is_none() {
        return Err("Recovery needs --save FRESH".into());
    }
    if args.first().is_some_and(|arg| arg == "--help") {
        println!(
            "Engineering check: netrunner-driver [decline|keep|rez|unrezzed|jack-out] [2|3] [--save PATH] [--recover-from ACTIVE] [--stop-after COUNT] [--view corp|runner]\nNative play uses netrunner-tabletop and requires prepared artwork."
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
    let mut game = Play::new(
        credits,
        save.map(|path| SaveOptions {
            path,
            recover_from: recovery,
        }),
    );
    game.admit();
    if let Admission::Refused(reason) = game.admission() {
        return Err(reason.clone());
    }
    let mut steps = 0;
    loop {
        let corp = game.observe(Player::Corp).ok_or("Match not admitted")?;
        if steps == limit || (json && corp.run_result.is_some()) {
            println!(
                "{}",
                serde_json::to_string(&game.observe(viewer)).map_err(|e| e.to_string())?
            );
            return Ok(());
        }
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
        let runner = game.observe(Player::Runner).ok_or("Match not admitted")?;
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
        game.choose_for(decision.actor, decision.revision, action)?;
        steps += 1;
    }
}
