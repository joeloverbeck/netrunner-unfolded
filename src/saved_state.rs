//! Netrunner's adapter onto the shared save record.
//!
//! `unfolded_record_replacement::save` owns the envelope, its checksum, the write
//! baseline, the unchanged shortcut, validation of the previous record, the
//! regular-file check and recovery into a fresh destination. This file keeps the
//! engine and content identities, what a match payload is and whether it resumes,
//! the launch's content requirement, when the current record is archived (a new
//! match) and the wording of Netrunner's refusals. Busy, too large, malformed
//! and file-system failures keep the shared module's words.
use crate::game::{Checkpoint, Match, Player};
use crate::policies::Policy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, path::PathBuf};
use unfolded_record_replacement::save::{self, Baseline, Format, Identity, Write};

const LIMIT: u64 = 1024 * 1024;
const TURN_ENGINE: &str = "ordinary-turns-10";
const INTRO_ENGINE: &str = "introductory-match-2";
const TURN_CONTENT: &str = include_str!("../resources/turns-assets.json");
/// The digest of the content every save requires.
static CONTENT: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| format!("{:x}", Sha256::digest(TURN_CONTENT.as_bytes())));

#[derive(Clone)]
pub struct SaveOptions {
    pub path: PathBuf,
    /// Recover the previous record of this active path into a fresh destination.
    pub recover_from: Option<PathBuf>,
}

#[derive(Debug)]
struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl From<save::Error> for Error {
    fn from(error: save::Error) -> Self {
        Self(match error {
            save::Error::Unsupported(_) => "Unsupported save format, engine or required content. Use a compatible build and artwork; this record was left unchanged.".into(),
            save::Error::Checksum => "Save checksum mismatch. This record was left unchanged.".into(),
            save::Error::Stale | save::Error::Missing | save::Error::Occupied => "The save changed, disappeared, or its destination is already occupied. Reopen it before continuing.".into(),
            save::Error::Orphaned => "A previous record exists without an active save. Recover it explicitly into a fresh path.".into(),
            save::Error::SameRecord => "Recovery needs a fresh destination distinct from the active and previous records.".into(),
            save::Error::NotRegularFile => "The save must be a regular file, not a link or directory.".into(),
            other => other.to_string(),
        })
    }
}

/// The introductory opponent, named so a changed policy is refused.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Opponent {
    policy: Policy,
    identity: String,
}

/// What a Netrunner save holds.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Saved {
    Turns(Checkpoint),
    Introductory {
        human: Player,
        #[serde(rename = "match")]
        checkpoint: Checkpoint,
        opponent: Opponent,
    },
}

impl Saved {
    fn of(game: &Match, human: Option<Player>, policy: Option<Policy>) -> Result<Self, Error> {
        let checkpoint = game.checkpoint();
        if !game.is_introductory() {
            return Ok(Self::Turns(checkpoint));
        }
        let human =
            human.ok_or_else(|| Error("Introductory saves require a human seat.".into()))?;
        let policy =
            policy.ok_or_else(|| Error("Introductory saves require an opponent policy.".into()))?;
        Ok(Self::Introductory {
            human,
            checkpoint,
            opponent: Opponent {
                policy,
                identity: policy.identity(),
            },
        })
    }
    /// The match, its human seat and its opponent policy.
    fn resume(self) -> Result<(Match, Option<Player>, Option<Policy>), String> {
        let introductory = matches!(self, Self::Introductory { .. });
        let (checkpoint, human, policy) = match self {
            Self::Turns(checkpoint) => (checkpoint, None, None),
            Self::Introductory {
                human,
                checkpoint,
                opponent,
            } => {
                if opponent.policy.role() != human.other()
                    || opponent.identity != opponent.policy.identity()
                {
                    return Err("Unsupported or changed saved opponent policy.".into());
                }
                (checkpoint, Some(human), Some(opponent.policy))
            }
        };
        let game = Match::resume(checkpoint)?;
        if game.is_introductory() != introductory {
            return Err("Save engine does not match the game payload.".into());
        }
        Ok((game, human, policy))
    }
}

struct Netrunner;
impl Netrunner {
    fn identity_for(engine: &str) -> Identity {
        [
            ("kind", "netrunner-match"),
            ("format", "1"),
            ("engine", engine),
            ("content", CONTENT.as_str()),
        ]
        .into_iter()
        .collect()
    }
}
impl Format for Netrunner {
    type Payload = Saved;
    const LIMIT: u64 = LIMIT;
    fn identity(&self, saved: &Saved) -> Identity {
        Self::identity_for(match saved {
            Saved::Turns(_) => TURN_ENGINE,
            Saved::Introductory { .. } => INTRO_ENGINE,
        })
    }
    fn accepts(&self, identity: &Identity) -> bool {
        [TURN_ENGINE, INTRO_ENGINE]
            .into_iter()
            .any(|engine| *identity == Self::identity_for(engine))
    }
    fn validate(&self, saved: &Saved) -> Result<(), String> {
        saved.clone().resume().map(|_| ())
    }
}

pub(crate) struct Record {
    path: PathBuf,
    baseline: Baseline,
    human: Option<Player>,
    policy: Option<Policy>,
}
impl Record {
    pub fn open(options: &SaveOptions, fresh: Match) -> Result<(Self, Match), String> {
        let turns = fresh.has_turns();
        Self::open_inner(options, Some(fresh), turns, None, None).map_err(|error| format!("{error} To recover a valid previous record, use --recover-from ACTIVE --save FRESH. Close and reopen after repair."))
    }
    pub fn open_introductory(
        options: &SaveOptions,
        fresh: Match,
        human: Player,
        policy: Policy,
    ) -> Result<(Self, Match), String> {
        Self::open_inner(options, Some(fresh), true, Some(human), Some(policy))
            .map_err(|error| error.to_string())
    }
    pub fn human(&self) -> Option<Player> {
        self.human
    }
    pub fn policy(&self) -> Option<Policy> {
        self.policy
    }
    pub fn archive_introductory(
        &mut self,
        game: &Match,
        human: Player,
        policy: Policy,
    ) -> Result<PathBuf, String> {
        let previous = (self.human, self.policy);
        self.human = Some(human);
        self.policy = Some(policy);
        let result = self
            .write(game, Write::Archive)
            .and_then(|path| {
                path.ok_or_else(|| Error("The previous match was not archived.".into()))
            })
            .map_err(|error| {
                format!("Could not start a new match: {error} Close and reopen after repair.")
            });
        if result.is_err() {
            (self.human, self.policy) = previous;
        }
        result
    }
    pub fn open_turns(options: &SaveOptions) -> Result<(Self, Match), String> {
        Self::open_inner(options, None, true, None, None).map_err(|error| error.to_string())
    }
    fn open_inner(
        options: &SaveOptions,
        fresh: Option<Match>,
        require_turns: bool,
        human: Option<Player>,
        policy: Option<Policy>,
    ) -> Result<(Self, Match), Error> {
        let require_introductory = fresh.as_ref().map(Match::is_introductory);
        let suits = |game: &Match| {
            require_turns == game.has_turns()
                && require_introductory.is_none_or(|required| required == game.is_introductory())
        };
        let mut record = Self {
            path: options.path.clone(),
            baseline: Baseline::default(),
            human,
            policy,
        };
        if let Some(source) = &options.recover_from {
            let (saved, baseline) =
                save::recover(&Netrunner, source, &options.path, |saved: &Saved| {
                    let (game, _, _) = saved.clone().resume().map_err(Error)?;
                    if !suits(&game) {
                        return Err(Error(
                            "Recovery requires the same game content as this launch.".into(),
                        ));
                    }
                    Ok(())
                })?;
            return record.resume(saved, baseline);
        }
        if let Some((saved, baseline)) = save::load(&Netrunner, &options.path)? {
            let (record, game) = record.resume(saved, baseline)?;
            if !suits(&game) {
                return Err(Error(
                    "The save requires different game content than this launch.".into(),
                ));
            }
            return Ok((record, game));
        }
        let game = fresh.ok_or_else(|| {
            Error(
                "An existing ordinary-turn engineering save is required. No game was created."
                    .into(),
            )
        })?;
        record.write(&game, Write::Create)?;
        Ok((record, game))
    }
    /// This record, continuing the saved match under `baseline`.
    fn resume(mut self, saved: Saved, baseline: Baseline) -> Result<(Self, Match), Error> {
        let (game, human, policy) = saved.resume().map_err(Error)?;
        self.baseline = baseline;
        self.human = human;
        self.policy = policy;
        Ok((self, game))
    }
    pub fn commit(&mut self, game: &Match) -> Result<(), String> {
        self.write(game, Write::Replace).map(|_| ()).map_err(|error| format!("Could not save: {error} Play stopped at the last committed decision. Close and reopen after repair."))
    }
    fn write(&mut self, game: &Match, mode: Write) -> Result<Option<PathBuf>, Error> {
        let saved = Saved::of(game, self.human, self.policy)?;
        Ok(save::write(
            &Netrunner,
            &self.path,
            &mut self.baseline,
            &saved,
            mode,
        )?)
    }
}
