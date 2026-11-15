use netrunner_unfolded::game::Player;
use netrunner_unfolded::play::SaveOptions;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let agent_seat = arguments.iter().any(|arg| arg == "--agent-seat");
    let mut args = arguments.into_iter();
    let mut root = netrunner_unfolded::assets::default_turns_root();
    let mut save = None;
    let mut recover_from = None;
    let mut turns = false;
    let mut seat = None;
    let mut socket = None;
    while let Some(flag) = args.next() {
        if flag == "--agent-seat" {
            continue;
        }
        if flag == "--turns" {
            turns = true;
            continue;
        }
        if flag == "--help" {
            println!(
                "netrunner-tabletop [--assets PREPARED_COLLECTION] [--save PATH] [--seat corp|runner] [--recover-from ACTIVE]\nChoose Corp or Runner and play the System Gateway introductory decks to 6 agenda points. With --save, reopen the same path to resume the saved hand and role.\nEngineering inspection: --turns --seat corp|runner --save EXISTING_SAVE loads a driver-prepared continuation.\nAgent input: --agent-seat [--socket PATH] uses the same rendered table without a desktop window. Without --socket, requests and responses are JSON lines on stdin/stdout.\nRecover ACTIVE.previous into an unused --save destination. Required artwork is admitted before a match is loaded or saved."
            );
            return std::process::ExitCode::SUCCESS;
        }
        let Some(value) = args.next() else {
            return usage(agent_seat);
        };
        if value.starts_with("--") {
            return usage(agent_seat);
        }
        match flag.as_str() {
            "--assets" => {
                root = value.into();
            }
            "--seat" => {
                seat = Some(match value.as_str() {
                    "corp" => Player::Corp,
                    "runner" => Player::Runner,
                    _ => return usage(agent_seat),
                })
            }
            "--save" if save.is_none() => save = Some(value.into()),
            "--recover-from" if recover_from.is_none() => recover_from = Some(value.into()),
            "--socket" if socket.is_none() => socket = Some(std::path::PathBuf::from(value)),
            _ => return usage(agent_seat),
        }
    }
    if (recover_from.is_some() || turns) && save.is_none() || socket.is_some() && !agent_seat {
        return usage(agent_seat);
    }
    let options = save.map(|path| SaveOptions { path, recover_from });
    let result = if agent_seat {
        netrunner_unfolded::tabletop::seat::run(&root, options, turns, seat, socket.as_deref())
    } else if turns {
        netrunner_unfolded::tabletop::run_turns_with_save(
            &root,
            options.unwrap(),
            seat.unwrap_or(Player::Corp),
        )
    } else {
        netrunner_unfolded::tabletop::run_introductory_with_save(&root, options, seat)
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cannot play: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn usage(agent_seat: bool) -> std::process::ExitCode {
    let message = "Usage: netrunner-tabletop [--assets PREPARED_COLLECTION] [--save PATH] [--recover-from ACTIVE] [--turns --seat corp|runner] [--agent-seat [--socket PATH]]. Turns requires an existing --save; socket requires --agent-seat; recovery requires an unused destination.";
    if agent_seat {
        let _: Result<(), String> = unfolded_tabletop::seat::refuse(message.into());
    }
    eprintln!("{message}");
    std::process::ExitCode::FAILURE
}
