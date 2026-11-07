use netrunner_unfolded::assets;
use std::path::Path;

fn main() -> std::process::ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command, root] if command == "check" => assets::inspect(Path::new(root)).map(|set| {
            println!(
                "Verified {} required images at {}. Renderer admission remains required.",
                set.images.len(),
                set.root.display()
            )
        }),
        [command, pdf, tokens, output] if command == "prepare" => {
            assets::prepare(Path::new(pdf), Path::new(tokens), Path::new(output))
                .map(|()| println!("Prepared artwork: {output}"))
        }
        _ => {
            println!(
                "netrunner-assets prepare PDF TOKEN_COLLECTION OUTPUT\nnetrunner-assets check PREPARED_COLLECTION\n\nInputs and outputs stay outside Git. Requires Poppler's pdftoppm for preparation. Gameplay is offline."
            );
            return if args.is_empty() || args == ["--help"] {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            };
        }
    };
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Artwork: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
