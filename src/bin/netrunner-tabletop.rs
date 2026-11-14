use netrunner_unfolded::game::Player;
use netrunner_unfolded::play::SaveOptions;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let mut root = netrunner_unfolded::assets::default_root();
    let mut save = None;
    let mut recover_from = None;
    let mut turns = false;
    let mut seat = Player::Corp;
    let mut explicit_root = false;
    while let Some(flag) = args.next() {
        if flag == "--turns" {
            turns = true;
            continue;
        }
        if flag == "--help" {
            println!(
                "netrunner-tabletop [--assets PREPARED_COLLECTION] [--save PATH] [--recover-from ACTIVE]\nPlay Corp through the prepared Brân run. With --save, reopen the same path to resume.\nEngineering inspection: --turns --seat corp|runner --save EXISTING_SAVE. This loads a driver-prepared continuation only.\nRecover ACTIVE.previous into an unused --save destination. Required artwork is admitted before a match is loaded or saved."
            );
            return std::process::ExitCode::SUCCESS;
        }
        let Some(value) = args.next() else {
            return usage();
        };
        match flag.as_str() {
            "--assets" => {
                root = value.into();
                explicit_root = true;
            }
            "--seat" => {
                seat = match value.as_str() {
                    "corp" => Player::Corp,
                    "runner" => Player::Runner,
                    _ => return usage(),
                }
            }
            "--save" if save.is_none() => save = Some(value.into()),
            "--recover-from" if recover_from.is_none() => recover_from = Some(value.into()),
            _ => return usage(),
        }
    }
    if (recover_from.is_some() || turns) && save.is_none() || !turns && seat != Player::Corp {
        return usage();
    }
    if turns && !explicit_root {
        root = netrunner_unfolded::assets::default_turns_root();
    }
    let options = save.map(|path| SaveOptions { path, recover_from });
    let result = if turns {
        netrunner_unfolded::tabletop::run_turns_with_save(&root, options.unwrap(), seat)
    } else {
        netrunner_unfolded::tabletop::run_with_save(&root, options)
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cannot play: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn usage() -> std::process::ExitCode {
    eprintln!(
        "Usage: netrunner-tabletop [--assets PREPARED_COLLECTION] [--save PATH] [--recover-from ACTIVE] [--turns --seat corp|runner]. Turns requires an existing --save; recovery requires an unused destination."
    );
    std::process::ExitCode::FAILURE
}
