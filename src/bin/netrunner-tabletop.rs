fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let root = match args.as_slice() {
        [] => netrunner_unfolded::assets::default_root(),
        [flag, path] if flag == "--assets" => path.into(),
        [flag] if flag == "--help" => {
            println!(
                "netrunner-tabletop [--assets PREPARED_COLLECTION]\nPlay Corp through the prepared Brân run. Required artwork is checked before a match exists. No saves are written."
            );
            return std::process::ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("Usage: netrunner-tabletop [--assets PREPARED_COLLECTION]");
            return std::process::ExitCode::FAILURE;
        }
    };
    match netrunner_unfolded::tabletop::run(&root) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cannot play: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
