//! Local, bounded diagnostics; never loads artwork or a native save.
use netrunner_unfolded::diagnostics::{self, ComparisonPlan, Identity, Limits};
use std::path::Path;

fn main() -> diagnostics::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["identity"] => println!("{}", serde_json::to_string_pretty(&Identity::current()?)?),
        ["record", seed, directory] => println!("{}", serde_json::to_string_pretty(&diagnostics::record_introductory(seed.parse()?, Path::new(directory), Limits::default())?)?),
        ["source-case", directory] => {
            let (summary, view) = diagnostics::record_source_case(Path::new(directory))?;
            if (view.corp_credits, view.runner_credits, view.clicks[1], view.run.as_ref().map(|r| r.hosted_credits)) != (0, 4, 3, Some(3)) {
                return Err("Source-case arithmetic differs from the independent expected answer.".into());
            }
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        ["reproduce", record] => println!("{}", diagnostics::reproduce(Path::new(record))?),
        ["pilot", directory] => println!("{}", serde_json::to_string_pretty(&diagnostics::run_comparison(&ComparisonPlan::pilot(), Path::new(directory))?)?),
        ["compare", declaration, directory] => {
            let bytes = unfolded_record_replacement::read(Path::new(declaration), 65536, |s| -> Box<dyn std::error::Error> { s.into() })?;
            let plan = serde_json::from_slice(&bytes)?;
            println!("{}", serde_json::to_string_pretty(&diagnostics::run_comparison(&plan, Path::new(directory))?)?);
        }
        _ => return Err("Use identity | record SEED FRESH-DIR | source-case FRESH-DIR | reproduce PRIVATE.jsonl | pilot FRESH-DIR | compare PLAN.json FRESH-DIR".into()),
    }
    Ok(())
}
