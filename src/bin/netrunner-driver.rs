//! Asset-independent engineering probe of the production match boundary.
use netrunner_unfolded::game::{
    Action, CardKind, EngineeringCard, EngineeringPosition, Player, Server, Zone, baseline_response,
};

use netrunner_unfolded::play::{Admission, Play, SaveOptions};

/// One bounded, read-only policy call for native verification. The caller supplies
/// exactly the rendered seat's observation, never a save or authoritative world.
fn baseline_choice() -> Result<(), String> {
    use std::io::Read;
    #[derive(serde::Deserialize)]
    #[serde(bound(deserialize = "'de: 'static"))]
    struct SeatObservation {
        observation: netrunner_unfolded::game::Observation,
    }
    let args: Vec<_> = std::env::args().skip(2).collect();
    if args.len() != 1 {
        return Err("Use --baseline-choice PATH for one seat observation JSON.".into());
    }
    let mut input = String::new();
    std::fs::File::open(&args[0])
        .map_err(|error| error.to_string())?
        .take(1024 * 1024 + 1)
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    if input.len() > 1024 * 1024 {
        return Err("The seat observation exceeds the 1 MiB diagnostic limit.".into());
    }
    // Decoded static view strings live until this bounded, short-lived process
    // exits. It is never a daemon, authoritative state reader or cache.
    let input: &'static str = Box::leak(input.into_boxed_str());
    let seat: SeatObservation = serde_json::from_str(input).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&baseline_response(&seat.observation))
            .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> Result<(), String> {
    if std::env::args().nth(1).as_deref() == Some("--baseline-choice") {
        return baseline_choice();
    }
    let mut args = Vec::new();
    let mut input = std::env::args().skip(1);
    let mut save = None;
    let mut recovery = None;
    let mut limit = usize::MAX;
    let mut viewer = Player::Corp;
    let mut json = false;
    let mut turns = None;
    let mut seed = 42_u64;
    let mut stop_at = None;
    while let Some(arg) = input.next() {
        match arg.as_str() {
            "--engineering-turns" => turns = Some(input.next().ok_or("Missing engineering case")?),
            "--stop-at" => stop_at = Some(input.next().ok_or("Missing phase prefix")?),
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
            "draw20" => position = EngineeringPosition::draw_economy(seed),
            "economy19" => position = EngineeringPosition::turn_income(seed),
            "income18" | "tread18" => {
                position = EngineeringPosition::run_income(seed, case == "tread18")
            }
            "breakers17" => position = EngineeringPosition::breaker_run(seed),
            "damage16" | "karuna16" => {
                position = EngineeringPosition::damage_run(seed, case == "karuna16")
            }
            "hq14" => position = EngineeringPosition::hq_access(seed),
            "central15" | "winning15" => {
                position = EngineeringPosition::central_access(seed, case == "winning15")
            }
            "hub15" => {
                position.corp_credits = 0;
                position.shuffle_decks = false;
                position.cards = vec![
                    EngineeringCard {
                        kind: CardKind::SuperconductingHub,
                        zone: Zone::Root(Server::Remote(1)),
                        advancement: 3,
                        installed_turn: Some(1),
                        rezzed: false,
                    },
                    EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
                    EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Rd),
                    EngineeringCard::new(CardKind::HedgeFund, Zone::Rd),
                ];
            }
            "run13" => {
                position.active = Player::Runner;
                position.corp_credits = 3;
                position.runner_credits = 8;
                position.shuffle_decks = false;
                position.cards = vec![
                    EngineeringCard::new(CardKind::Overclock, Zone::Grip),
                    EngineeringCard::new(CardKind::Palisade, Zone::Hq),
                    EngineeringCard::new(CardKind::OffworldOffice, Zone::Hq),
                    EngineeringCard::new(CardKind::HedgeFund, Zone::Hq),
                    EngineeringCard::new(CardKind::SeamlessLaunch, Zone::Rd),
                ];
                for (kind, zone, rezzed) in [
                    (CardKind::Whitespace, Zone::Protecting(Server::Hq), true),
                    (CardKind::Bran, Zone::Protecting(Server::Hq), true),
                    (CardKind::Cleaver, Zone::Rig, false),
                    (CardKind::Unity, Zone::Rig, false),
                    (CardKind::DocklandsPass, Zone::Rig, false),
                ] {
                    position.cards.push(EngineeringCard {
                        kind,
                        zone,
                        rezzed,
                        installed_turn: Some(1),
                        advancement: 0,
                    });
                }
            }
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
            if step == limit
                || (case == "draw20" && view.turn_number >= 5)
                || (case == "economy19" && view.turn_number >= 8 && view.phase == "Action phase")
                || (case == "income18"
                    && view.turn_number >= 4
                    && view.run.is_none()
                    && view.central_runs.contains(&Server::Archives)
                    && view
                        .rig
                        .iter()
                        .any(|c| c.printing == Some("30014") && c.hosted_credits == 0))
                || (case == "tread18"
                    && view.run_result.is_some()
                    && view.heap.iter().any(|c| c.printing == Some("30012")))
                || (case == "breakers17"
                    && view.run_result.is_some()
                    && view.heap.iter().any(|c| c.printing == Some("30029")))
                || stop_at.as_ref().is_some_and(|prefix| {
                    view.phase
                        .to_lowercase()
                        .starts_with(&prefix.to_lowercase())
                })
                || (case == "hq14" && view.turn_number >= 4 && view.phase.starts_with("Breach HQ"))
                || (matches!(
                    case.as_str(),
                    "central15" | "winning15" | "damage16" | "karuna16"
                ) && view.run_result.is_some())
                || view.match_winner.is_some()
                || (case == "run13"
                    && (view.run_result.is_some()
                        || view
                            .run
                            .as_ref()
                            .is_some_and(|r| r.server_approach || r.hq_access_remaining.is_some())))
            {
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
            let action = if case == "draw20" {draw20_witness(&acting)} else if case == "economy19" {economy19_witness(&acting)} else if matches!(case.as_str(), "income18" | "tread18") {income18_witness(&acting, case == "tread18")} else if case == "breakers17" {breakers17_witness(&acting)} else if matches!(case.as_str(), "damage16" | "karuna16") {damage16_witness(&acting, case == "karuna16")} else if case == "hub15" && acting.phase.starts_with("Turn start") { Some(Action::PassPriority) } else if matches!(case.as_str(),"central15"|"winning15") {central15_witness(&acting)} else if case == "hq14" {hq14_witness(&acting)} else if case == "run13" {run13_witness(&acting)} else if case == "scoring" {
                decision.choices.iter().find(|a| matches!(a,Action::PlayCard(id) if acting.hand.iter().any(|c| c.instance == Some(*id) && c.printing == Some("30040")))).copied().or_else(|| baseline_response(&acting))
            } else if case == "discard" && decision.choices.contains(&Action::Purge) { Some(Action::Purge) }
            else { baseline_response(&acting) }.ok_or("No lawful baseline action")?;
            game.choose_for(decision.actor, decision.revision, action)?;
        }
        return Err("Engineering decision budget reached; game remains resumable, no artificial result assigned.".into());
    }
    if args.first().is_some_and(|arg| arg == "--help") {
        println!(
            "Engineering check: netrunner-driver [decline|keep|rez|unrezzed|jack-out] [2|3] [--save PATH] [--recover-from ACTIVE] [--stop-after COUNT] [--view corp|runner]\nBounded turn preparation: --engineering-turns economy|scoring|runner|discard|install|run13|hq14|central15|winning15|hub15|damage16|karuna16|breakers17|income18|tread18|economy19|draw20 --seed NUMBER --save PATH --stop-after COUNT [--stop-at PHASE_PREFIX]\nRead-only native verification: --baseline-choice PATH reads a seat observation JSON and prints its lawful baseline action.\nNative play uses netrunner-tabletop and requires prepared artwork."
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
        let view = if corp.decision.is_some() {
            corp
        } else {
            runner
        };
        let decision = view.decision.as_ref().ok_or("No current decision")?;
        let choices = &decision.choices;
        let action = if choices.contains(&Action::DeclineInstall) {
            if branch == "decline" {
                Some(Action::DeclineInstall)
            } else {
                choices
                    .iter()
                    .find(|a| matches!(a, Action::InstallInward(_)))
                    .copied()
            }
        } else if choices.contains(&Action::FinishInstall) {
            if branch != "keep" {
                choices
                    .iter()
                    .find(|a| matches!(a, Action::TrashInstalled(_)))
                    .copied()
                    .or(Some(Action::FinishInstall))
            } else {
                Some(Action::FinishInstall)
            }
        } else if branch == "rez" && choices.iter().any(|a| matches!(a, Action::Rez(_))) {
            choices
                .iter()
                .find(|a| matches!(a, Action::Rez(_)))
                .copied()
        } else if branch == "jack-out" && choices.contains(&Action::JackOut) {
            Some(Action::JackOut)
        } else {
            [Action::PassPriority, Action::Continue, Action::FinishAccess]
                .into_iter()
                .find(|a| choices.contains(a))
        }
        .ok_or("The requested Brân witness has no legal choice")?;
        let decision = view.decision.ok_or("No current decision.")?;
        game.choose_for(decision.actor, decision.revision, action)?;
        steps += 1;
    }
}

// Explicit source-derived witness, never the opponent policy. The first run
// earns Carmen's discount through the real successful-run transition.
// Deterministic engineering itinerary, not a strategy or alternate game path.
fn income18_witness(view: &netrunner_unfolded::game::Observation, tread: bool) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.phase == "Action phase" {
        if view.active_player == Some(Player::Corp) {
            return Some(Action::GainCredit);
        }
        if tread {
            return view
                .hand
                .iter()
                .find(|c| c.printing == Some("30012"))
                .and_then(|c| c.instance)
                .map(Action::PlayCard);
        }
        let red = view
            .rig
            .iter()
            .find(|c| c.printing == Some("30018"))?
            .instance?;
        let targets = if view.turn_number == 2 {
            vec![Server::Hq, Server::Rd]
        } else {
            vec![Server::Archives]
        };
        if let Some(server) = targets.into_iter().find(|s| !view.central_runs.contains(s)) {
            return Some(Action::RunRedTeam { card: red, server });
        }
        if let Some(penny) = view
            .rig
            .iter()
            .find(|c| c.printing == Some("30014") && c.hosted_credits > 0)
        {
            return Some(Action::TakePennyshaver(penny.instance?));
        }
        return Some(Action::GainCredit);
    }
    if tread {
        if choices.contains(&Action::RunServer(Server::Hq)) {
            return Some(Action::RunServer(Server::Hq));
        }
        for action in choices {
            if matches!(action, Action::InstallInward(_) | Action::Rez(_)) {
                return Some(*action);
            }
            if let Action::TrashInstalled(id) = action
                && view
                    .servers
                    .iter()
                    .flat_map(|s| &s.ice)
                    .any(|c| c.instance == Some(*id) && c.printing == Some("30039"))
            {
                return Some(*action);
            }
        }
        if choices.contains(&Action::FinishInstall) {
            return Some(Action::FinishInstall);
        }
        if let Some(action) = choices.iter().find(|a| matches!(a, Action::Break { .. })) {
            return Some(*action);
        }
        if choices.contains(&Action::PayRunCredits(0)) {
            return Some(Action::PayRunCredits(0));
        }
    }
    [
        Action::PassPriority,
        Action::Continue,
        Action::FinishAccess,
        Action::AccessHq,
        Action::AccessRd,
    ]
    .into_iter()
    .find(|a| choices.contains(a))
    .or_else(|| choices.first().copied())
}

fn breakers17_witness(view: &netrunner_unfolded::game::Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.phase == "Action phase" {
        if let Some(carmen) = view.hand.iter().find(|c| c.printing == Some("30015")) {
            return Some(if view.run_result.is_none() {
                Action::RunServer(Server::Archives)
            } else {
                Action::InstallCard(carmen.instance?)
            });
        }
        if let Some(event) = view.hand.iter().find(|c| c.printing == Some("30029")) {
            return Some(Action::PlayCard(event.instance?));
        }
    }
    if choices.contains(&Action::RunServer(Server::Hq)) {
        return Some(Action::RunServer(Server::Hq));
    }
    if choices.contains(&Action::FinishInstall) {
        return Some(Action::FinishInstall);
    }
    if let Some(action) = choices.iter().find(|a| **a == Action::PayRunCredits(1)) {
        return Some(*action);
    }
    if let Some(run) = &view.run
        && run.encounter_paid
    {
        let printing = match run.ice.as_ref()?.printing? {
            "30047" => "30015",
            "30072" => "30032",
            "30046" => "30026",
            _ => return None,
        };
        let breaker = view
            .rig
            .iter()
            .find(|c| c.printing == Some(printing))?
            .instance?;
        if let Some(action) = choices
            .iter()
            .find(|a| matches!(a,Action::Break{breaker:id,..} if *id==breaker))
        {
            return Some(*action);
        }
        if run.subroutines.iter().any(|s| !s.broken) && choices.contains(&Action::Boost(breaker)) {
            return Some(Action::Boost(breaker));
        }
    }
    if choices.contains(&Action::Continue) {
        return Some(Action::Continue);
    }
    if choices.contains(&Action::PassPriority) {
        return Some(Action::PassPriority);
    }
    baseline_response(view)
}

// Source-derived #7 witness only; every action uses the same production
// controller and legal decisions. This is not the opponent policy.
fn run13_witness(view: &netrunner_unfolded::game::Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if let Some(event) = view.hand.iter().find(|c| c.printing == Some("30029"))
        && let Some(id) = event.instance
        && choices.contains(&Action::PlayCard(id))
    {
        return Some(Action::PlayCard(id));
    }
    if choices.contains(&Action::RunServer(Server::Hq)) {
        return Some(Action::RunServer(Server::Hq));
    }
    for action in choices {
        if matches!(action, Action::InstallInward(_) | Action::Rez(_)) {
            return Some(*action);
        }
        if let Action::TrashInstalled(id) = action
            && view
                .servers
                .iter()
                .flat_map(|s| &s.ice)
                .any(|c| c.instance == Some(*id) && c.printing == Some("30039"))
        {
            return Some(*action);
        }
    }
    if choices.contains(&Action::FinishInstall) {
        return Some(Action::FinishInstall);
    }
    if choices.contains(&Action::PayRunCredits(1)) {
        return Some(Action::PayRunCredits(1));
    }
    if let Some(run) = &view.run
        && let Some(ice) = &run.ice
    {
        let mask = match ice.printing {
            Some("30072") => 1,
            Some("30074") => 2,
            _ => 0,
        };
        if let Some(action) = choices
            .iter()
            .find(|a| matches!(a,Action::Break{subroutines,..}if *subroutines==mask))
        {
            return Some(*action);
        }
    }
    if choices.contains(&Action::Continue) {
        return Some(Action::Continue);
    }
    choices
        .contains(&Action::PassPriority)
        .then_some(Action::PassPriority)
}

// Independent source-line witness, never used as the opponent policy.
fn hq14_witness(view: &netrunner_unfolded::game::Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    for action in [
        Action::SpendManegarmClicks,
        Action::AccessRoot(0),
        Action::TrashAccessed,
        Action::StealAgenda,
        Action::FinishAccess,
        Action::AccessHq,
    ] {
        if choices.contains(&action) {
            return Some(action);
        }
    }
    if view.run.as_ref().is_some_and(|r| r.payment_cost.is_some()) {
        return choices
            .iter()
            .filter(|a| matches!(a, Action::PayRunCredits(_)))
            .max_by_key(|a| {
                if let Action::PayRunCredits(n) = a {
                    *n
                } else {
                    0
                }
            })
            .copied();
    }
    if view.turn_number == 3 {
        if view.agenda_points[0] == 2 && choices.contains(&Action::GainCredit) {
            return Some(Action::GainCredit);
        }
        return baseline_response(view);
    }
    if view.turn_number >= 4
        && let Some(run) = &view.run
        && run
            .ice
            .as_ref()
            .is_some_and(|c| c.printing == Some("30074"))
        && let Some(action) = choices
            .iter()
            .find(|a| matches!(a, Action::Break { subroutines: 1, .. }))
    {
        return Some(*action);
    }
    if view.turn_number == 2
        && view.run.is_none()
        && view.run_result.is_some()
        && choices.contains(&Action::GainCredit)
    {
        return Some(Action::GainCredit);
    }
    run13_witness(view)
}

// Source-defined run choice only; subsequent decisions use lawful observations.
fn central15_witness(view: &netrunner_unfolded::game::Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.run.is_none() {
        if let Some(action)=choices.iter().find(|a|matches!(a,Action::PlayCard(id) if view.hand.iter().any(|c|c.instance==Some(*id) && c.printing==Some("30028")))) { return Some(*action); }
        if choices.contains(&Action::RunServer(Server::Rd)) {
            return Some(Action::RunServer(Server::Rd));
        }
    }
    baseline_response(view)
}

// Source-derived witness requests remain ordinary validated choices. The native
// opponent continues to use baseline_response, not this engineering itinerary.
fn damage16_witness(view: &netrunner_unfolded::game::Observation, karuna: bool) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.phase == "Action phase" && !karuna {
        return choices.iter().find(|a| matches!(a, Action::PlayCard(id) if view.hand.iter().any(|c| c.instance == Some(*id) && c.printing == Some("30029")))).copied();
    }
    if choices.contains(&Action::RunServer(Server::Remote(1))) {
        return Some(Action::RunServer(Server::Remote(1)));
    }
    if view.phase.starts_with("Karunā") {
        return Some(Action::Continue);
    }
    baseline_response(view)
}

// Source-derived itinerary for #19, distinct from the opponent's policy. Every
// installation, optional activation and mandatory payout uses ordinary choices.
fn economy19_witness(view: &netrunner_unfolded::game::Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.phase == "Action phase" {
        for action in choices {
            if matches!(action, Action::TakeTelework(_) | Action::TakeRegolith(_)) {
                return Some(*action);
            }
        }
        for printing in if view.active_player == Some(Player::Corp) {
            &["30037", "30071"][..]
        } else {
            &["30027", "30033"][..]
        } {
            if let Some(action) = choices.iter().find(|a| matches!(a,Action::InstallCard(id) if view.hand.iter().any(|c| c.instance == Some(*id) && c.printing == Some(*printing)))) { return Some(*action); }
        }
        if let Some(action) = choices.iter().find(|a| matches!(a,Action::LoadSmartware(id) if view.rig.iter().any(|c| c.instance == Some(*id) && c.hosted_credits < 3))) { return Some(*action); }
        return Some(Action::GainCredit);
    }
    baseline_response(view)
}

// #20 source-derived itinerary; the native opponent remains the lawful baseline.
fn draw20_witness(view: &netrunner_unfolded::game::Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.phase == "Action phase" {
        if view.active_player == Some(Player::Corp) {
            return Some(Action::GainCredit);
        }
        let (printing, install) = match (view.turn_number, view.clicks[1]) {
            (2, 4 | 2) | (4, 2) => return Some(Action::Draw),
            (2, 3) => ("30034", true),
            (2, 1) => ("30020", false),
            (4, 4 | 1) => ("30021", false),
            _ => return Some(Action::GainCredit),
        };
        return choices
            .iter()
            .find(|action| match action {
                Action::InstallCard(id) if install => view
                    .hand
                    .iter()
                    .any(|c| c.instance == Some(*id) && c.printing == Some(printing)),
                Action::PlayCard(id) if !install => view
                    .hand
                    .iter()
                    .any(|c| c.instance == Some(*id) && c.printing == Some(printing)),
                _ => false,
            })
            .copied();
    }
    baseline_response(view)
}
