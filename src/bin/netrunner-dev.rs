//! Netrunner's optional development command composition; reader/check machinery
//! remains in the existing Arkham tools, never copied into the game.
use netrunner_unfolded::{
    game::{Action, Player},
    play::{Admission, Play, SaveOptions},
};
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const HELP: &str = "netrunner-dev read|read-batch|search|summary ARGS...\nnetrunner-dev record OUTPUT_DIR -- COMMAND...\nnetrunner-dev preflight [ASSETS [OUTPUT_DIR]]\nReaders share Arkham's 6000-byte bound. Set ARKHAM_TOOLS to its built executable.\nPreflight defaults to $XDG_DATA_HOME/netrunner-unfolded/turns-assets (or ~/.local/share),\nand a fresh local/verification/native-TIMESTAMP directory. Full output stays on disk.";

fn main() -> ExitCode {
    match run(env::args_os().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("netrunner-dev: {error}");
            ExitCode::FAILURE
        }
    }
}

fn tools() -> Result<PathBuf> {
    let path = env::var_os("ARKHAM_TOOLS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../arkham-unfolded/target/agent-tools/debug/arkham-tools")
        });
    if !path.is_file() {
        return Err(
            "Build Arkham's optional tools or set ARKHAM_TOOLS; see docs/agents/development.md"
                .into(),
        );
    }
    Ok(fs::canonicalize(path)?)
}

fn run(args: Vec<OsString>) -> Result<()> {
    let Some(command) = args.first().and_then(|s| s.to_str()) else {
        println!("{HELP}");
        return Ok(());
    };
    if matches!(command, "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let helper = tools()?;
    match command {
        "read" | "read-batch" | "search" | "summary" => {
            // The helper owns paging, hashes, omission markers and source-change refusal.
            // Refuse budget overrides rather than silently truncating its output.
            if args[1..]
                .iter()
                .any(|s| s.to_string_lossy().starts_with("--max-bytes"))
            {
                return Err("This entry point fixes the shared output budget at 6000 bytes".into());
            }
            let operation = match command {
                "read" => "read-context",
                "read-batch" => "read-context-batch",
                "search" => "search-context",
                _ => "check-summary",
            };
            if !Command::new(helper)
                .arg(operation)
                .args(&args[1..])
                .args(["--max-bytes", "6000"])
                .status()?
                .success()
            {
                return Err("Bounded reader failed; follow its diagnostic".into());
            }
        }
        "record" if args.len() >= 4 && args[2] == "--" => {
            let output = PathBuf::from(&args[1]);
            recorded(&helper, &output, &args[3..], &[])?;
        }
        "preflight" if args.len() <= 3 => {
            let assets = match args
                .get(1)
                .map(PathBuf::from)
                .or_else(|| env::var_os("NETRUNNER_ART_ROOT").map(PathBuf::from))
            {
                Some(path) => path,
                None => data_root()?.join("netrunner-unfolded/turns-assets"),
            };
            // Admit first: a missing/changed collection must not create a misleading run.
            netrunner_unfolded::assets::inspect_turns(&assets)?;
            let assets = fs::canonicalize(assets)?;
            let output = args.get(2).map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(format!(
                    "local/verification/native-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("clock")
                        .as_millis()
                ))
            });
            fresh(&output)?;
            let output = fs::canonicalize(output)?;
            let fixture = output.join("runner.json");
            ordinary_fixture(&fixture)?;
            fs::write(
                output.join("README.txt"),
                format!(
                    "Netrunner native preflight. See issue #37 and docs/agents/development.md.\nAssets: {}\nEach stage has check.log and run.json; use netrunner-dev summary STAGE/check.log.\nPASS is recorded only after every stage completes; captures require visual inspection.\n",
                    assets.display()
                ),
            )?;
            let envs = [
                ("NETRUNNER_ART_ROOT", assets.as_os_str()),
                ("NETRUNNER_TURNS_SAVE", fixture.as_os_str()),
                ("NETRUNNER_SEAT", std::ffi::OsStr::new("runner")),
            ];
            for (name, test, ignored) in [
                (
                    "growth",
                    "tabletop::tests::fitted_table_labels_have_real_text_clearance_in_both_seats",
                    false,
                ),
                (
                    "actions",
                    "tabletop::seat::tests::rendered_mat_atlas_preserves_seat_actions_and_public_count_updates",
                    true,
                ),
                (
                    "inspection",
                    "tabletop::tests::actual_renderer_uploads_required_images_and_refuses_changed_pixels",
                    true,
                ),
            ] {
                let capture = output.join(format!("{name}.png"));
                let mut environment = envs.to_vec();
                environment.push(("NETRUNNER_RENDER_CAPTURE", capture.as_os_str()));
                let mut cmd: Vec<OsString> = [
                    "cargo",
                    "test",
                    "--locked",
                    "--features",
                    "tabletop",
                    "--lib",
                    test,
                    "--",
                    "--exact",
                    "--nocapture",
                ]
                .map(Into::into)
                .into();
                if ignored {
                    cmd.push("--ignored".into());
                }
                let stage = output.join(name);
                recorded(&helper, &stage, &cmd, &environment)?;
                let log = fs::read_to_string(stage.join("check.log"))?;
                if !log.contains("test result: ok. 1 passed;") {
                    return Err(format!(
                        "{name}: expected exactly one executed test; inspect retained log"
                    )
                    .into());
                }
                if ignored && !capture.is_file() {
                    return Err(format!("{name}: capture missing").into());
                }
            }
            fs::write(
                output.join("PASS"),
                "All three preflight stages passed. This is automated evidence, not human acceptance.\n",
            )?;
            println!("PREFLIGHT PASS {}", output.display());
        }
        _ => return Err(HELP.into()),
    }
    Ok(())
}

fn data_root() -> Result<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .ok_or_else(|| "Set NETRUNNER_ART_ROOT or provide ASSETS".into())
}

fn fresh(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(path)?; // Refuse reuse, including a symlink; never replace prior evidence.
    Ok(())
}

fn validate_record(value: &serde_json::Value) -> Result<()> {
    let before = value["source"]["before_sha256"].as_str();
    let after = value["source"]["after_sha256"].as_str();
    if value["status"] != "finished"
        || value["exit_code"] != 0
        || before.is_none()
        || before != after
    {
        return Err("Recorded command failed, is unfinished, or its source changed; inspect run.json/check.log".into());
    }
    Ok(())
}

fn recorded(
    helper: &Path,
    output: &Path,
    argv: &[OsString],
    environment: &[(&str, &std::ffi::OsStr)],
) -> Result<()> {
    if output.exists() {
        return Err(format!("Evidence path already exists: {}", output.display()).into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    // record-check owns process lifetime, complete logs and atomic completion records.
    let mut command = Command::new(helper);
    if !environment.is_empty() {
        // A caller's pile-probe mode must not replace the ordinary-save checks.
        command.env_remove("NETRUNNER_PILE_PROBE");
    }
    let status = command
        .arg("record-check")
        .arg("--output-dir")
        .arg(output)
        .arg("--")
        .args(argv)
        .envs(environment.iter().copied())
        .stdin(Stdio::null())
        .status()?;
    let value: serde_json::Value = serde_json::from_slice(&fs::read(output.join("run.json"))?)?;
    if !status.success() {
        return Err(format!("Command failed; evidence: {}", output.display()).into());
    }
    validate_record(&value)?;
    println!("PASS {}", output.display());
    Ok(())
}

fn ordinary_fixture(path: &Path) -> Result<()> {
    if path.exists() {
        return Err("Fixture destination must be fresh".into());
    }
    let mut play = Play::introductory_seeded(
        42,
        Some(SaveOptions {
            path: path.into(),
            recover_from: None,
        }),
        Player::Runner,
    );
    play.admit();
    if play.admission() != &Admission::Ready {
        return Err(format!("Fixture admission: {:?}", play.admission()).into());
    }
    for _ in 0..128 {
        if play.opponent_pending() {
            play.respond()?;
            continue;
        }
        let observation = play.observation().ok_or("Missing fixture observation")?;
        let decision = observation
            .decision
            .ok_or("Fixture ended before Runner action phase")?;
        if decision.choices.contains(&Action::GainCredit)
            && decision.choices.contains(&Action::Draw)
            && observation.clicks[1] >= 2
            && observation.stack_count > 0
        {
            return Ok(());
        }
        if !decision.choices.contains(&Action::KeepHand) {
            return Err("Unexpected fixture choice before Runner action phase".into());
        }
        play.choose(decision.revision, Action::KeepHand)?;
    }
    Err("Fixture setup exceeded 128 decisions".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_requires_success_and_unchanged_source() {
        let good = serde_json::json!({"status":"finished","exit_code":0,"source":{"before_sha256":"a","after_sha256":"a"}});
        assert!(validate_record(&good).is_ok());
        for change in [
            serde_json::json!("running"),
            serde_json::json!("invalidated"),
        ] {
            let mut value = good.clone();
            value["status"] = change;
            assert!(validate_record(&value).is_err());
        }
        let mut value = good.clone();
        value["exit_code"] = 1.into();
        assert!(validate_record(&value).is_err());
        let mut value = good;
        value["source"]["after_sha256"] = "b".into();
        assert!(validate_record(&value).is_err());
        assert!(validate_record(&serde_json::json!({"status":"finished","exit_code":0})).is_err());
    }
    #[test]
    fn fixture_is_ordinary_restartable_and_never_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("runner.json");
        ordinary_fixture(&file).unwrap();
        let before = fs::read(&file).unwrap();
        assert!(ordinary_fixture(&file).is_err());
        let mut reopened = Play::introductory(
            Some(SaveOptions {
                path: file.clone(),
                recover_from: None,
            }),
            None,
        )
        .unwrap();
        reopened.admit();
        assert_eq!(reopened.human(), Player::Runner);
        assert!(
            reopened
                .observation()
                .unwrap()
                .decision
                .unwrap()
                .choices
                .contains(&Action::Draw)
        );
        assert_eq!(fs::read(file).unwrap(), before);
    }
    #[test]
    fn evidence_directory_cannot_be_reused() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("run");
        fresh(&output).unwrap();
        fs::write(output.join("kept"), "evidence").unwrap();
        assert!(fresh(&output).is_err());
        assert_eq!(fs::read_to_string(output.join("kept")).unwrap(), "evidence");
    }
}
