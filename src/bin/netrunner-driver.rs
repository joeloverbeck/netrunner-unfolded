//! Asset-independent engineering probe of the production match boundary.
use netrunner_unfolded::game::{
    Action, CardKind, EngineeringCard, EngineeringPosition, Player, Server, Zone,
    baseline_response, fixed_runner_response,
};

use netrunner_unfolded::play::{Admission, Play, SaveOptions};

fn main() -> Result<(), String> {
    let mut args = Vec::new();
    let mut input = std::env::args().skip(1);
    let mut save = None;
    let mut recovery = None;
    let mut limit = usize::MAX;
    let mut viewer = Player::Corp;
    let mut json = false;
    let mut turns = None;
    let mut seed = 42_u64;
    while let Some(arg) = input.next() {
        match arg.as_str() {
            "--engineering-turns" => turns = Some(input.next().ok_or("Missing engineering case")?),
            "--seed" => {
                seed = input
                    .next()
                    .ok_or("Missing seed")?
                    .parse()
                    .map_err(|_| "Invalid seed")?
            }
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
    if let Some(case) = turns {
        if !args.is_empty() {
            return Err("Unexpected engineering arguments".into());
        }
        let mut position = EngineeringPosition::economy(seed);
        match case.as_str() {
            "economy" => {}
            "scoring" => {
                position.corp_credits = 0;
                position.shuffle_decks = false;
                position.cards.push(EngineeringCard {
                    kind: CardKind::OffworldOffice,
                    zone: Zone::Root(Server::Remote(1)),
                    advancement: 2,
                    installed_turn: Some(1),
                    rezzed: false,
                });
            }
            "runner" => {
                position.active = Player::Runner;
                position.runner_credits = 8;
            }
            "discard" => {
                position
                    .cards
                    .extend((0..4).map(|_| EngineeringCard::new(CardKind::HedgeFund, Zone::Hq)));
            }
            "install" => {
                position.active = Player::Runner;
                position.runner_credits = 3;
                position.cards.retain(|c| c.kind != CardKind::SureGamble);
            }
            _ => return Err(
                "Use economy, scoring, runner, discard or install as a bounded engineering case."
                    .into(),
            ),
        }
        let mut game = Play::engineering(
            position,
            save.map(|path| SaveOptions {
                path,
                recover_from: recovery,
            }),
            viewer,
        );
        game.admit();
        if let Admission::Refused(reason) = game.admission() {
            return Err(reason.clone());
        }
        for step in 0..=limit.min(1000) {
            let view = game.observe(viewer).ok_or("Match not admitted")?;
            if step == limit || view.match_winner.is_some() {
                println!(
                    "{}",
                    serde_json::to_string(&view).map_err(|error| error.to_string())?
                );
                return Ok(());
            }
            let acting = [Player::Corp, Player::Runner]
                .into_iter()
                .find_map(|player| game.observe(player).filter(|view| view.decision.is_some()))
                .ok_or("No ordinary decision")?;
            let decision = acting.decision.as_ref().unwrap();
            // The scoring witness follows the independent #7 economy line;
            // all requests still pass through the production legal boundary.
            let action = if case == "scoring" {
                decision.choices.iter().find(|a| matches!(a,Action::PlayCard(id) if acting.hand.iter().any(|c| c.instance == Some(*id) && c.printing == Some("30040")))).copied().or_else(|| baseline_response(&acting))
            } else if case == "discard" && decision.choices.contains(&Action::Purge) { Some(Action::Purge) }
            else { baseline_response(&acting) }.ok_or("No lawful baseline action")?;
            game.choose_for(decision.actor, decision.revision, action)?;
        }
        return Err("Engineering decision budget reached; game remains resumable, no artificial result assigned.".into());
    }
    if args.first().is_some_and(|arg| arg == "--help") {
        println!(
            "Engineering check: netrunner-driver [decline|keep|rez|unrezzed|jack-out] [2|3] [--save PATH] [--recover-from ACTIVE] [--stop-after COUNT] [--view corp|runner]\nBounded turn preparation: --engineering-turns economy|scoring|runner|discard|install --seed NUMBER --save PATH --stop-after COUNT\nNative play uses netrunner-tabletop and requires prepared artwork. Complete introductory setup remains unavailable."
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
