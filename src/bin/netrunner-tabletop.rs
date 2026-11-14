use netrunner_unfolded::play::SaveOptions;

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let mut root = netrunner_unfolded::assets::default_root();
    let mut save = None;
    let mut recover_from = None;
    while let Some(flag) = args.next() {
        if flag == "--help" {
            println!(
                "netrunner-tabletop [--assets PREPARED_COLLECTION] [--save PATH] [--recover-from ACTIVE]\nPlay Corp through the prepared Brân run. With --save, reopen the same path to resume. Recover ACTIVE.previous into an unused --save destination. Required artwork is admitted before a match is loaded or saved."
            );
            return std::process::ExitCode::SUCCESS;
        }
        let Some(value) = args.next() else {
            return usage();
        };
        match flag.as_str() {
            "--assets" => root = value.into(),
            "--save" if save.is_none() => save = Some(value.into()),
            "--recover-from" if recover_from.is_none() => recover_from = Some(value.into()),
            _ => return usage(),
        }
    }
    if recover_from.is_some() && save.is_none() {
        return usage();
    }
    match netrunner_unfolded::tabletop::run_with_save(
        &root,
        save.map(|path| SaveOptions { path, recover_from }),
    ) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cannot play: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn usage() -> std::process::ExitCode {
    eprintln!(
        "Usage: netrunner-tabletop [--assets PREPARED_COLLECTION] [--save PATH] [--recover-from ACTIVE]. Recovery requires an unused --save path."
    );
    std::process::ExitCode::FAILURE
}
