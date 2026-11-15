//! Netrunner admission policy around the shared locked replacement lifecycle.
use crate::game::{Checkpoint, Match, Player};
use crate::policies::Policy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};
use unfolded_record_replacement::{self as record_replacement, Replacement};

const LIMIT: u64 = 1024 * 1024;
const TURN_ENGINE: &str = "ordinary-turns-10";
const OLD_INTRO_ENGINE: &str = "introductory-match-1";
const INTRO_ENGINE: &str = "introductory-match-2";
const TURN_CONTENT: &str = include_str!("../resources/turns-assets.json");

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
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self(format!("Invalid save: {value}"))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u32,
    kind: String,
    engine: String,
    content: String,
    payload: serde_json::Value,
    checksum: String,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
impl Envelope {
    fn checksum(&self) -> Result<String, Error> {
        Ok(digest(&serde_json::to_vec(&(
            self.format,
            &self.kind,
            &self.engine,
            &self.content,
            &self.payload,
        ))?))
    }
}
fn encode(game: &Match, human: Option<Player>, policy: Option<Policy>) -> Result<Vec<u8>, Error> {
    let checkpoint = game.checkpoint();
    Match::resume(checkpoint.clone()).map_err(|reason| Error(reason.into()))?;
    let mut envelope = Envelope {
        format: 1,
        kind: "netrunner-match".into(),
        engine: if game.is_introductory() {
            INTRO_ENGINE
        } else {
            TURN_ENGINE
        }
        .into(),
        content: digest(TURN_CONTENT.as_bytes()),
        payload: if game.is_introductory() {
            serde_json::json!({
                "human": human.ok_or_else(|| Error("Introductory saves require a human seat.".into()))?,
                "match": checkpoint,
                "opponent": { "policy": policy.ok_or_else(|| Error("Introductory saves require an opponent policy.".into()))?, "identity": policy.unwrap().identity() },
            })
        } else {
            serde_json::to_value(checkpoint)?
        },
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    Ok(serde_json::to_vec(&envelope)?)
}
fn decode(bytes: &[u8]) -> Result<(Match, Option<Player>, Option<Policy>), Error> {
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.format != 1
        || envelope.kind != "netrunner-match"
        || !((envelope.engine == TURN_ENGINE
            || envelope.engine == INTRO_ENGINE
            || envelope.engine == OLD_INTRO_ENGINE)
            && envelope.content == digest(TURN_CONTENT.as_bytes()))
    {
        return Err(Error("Unsupported save format, engine or required content. Use a compatible build and artwork; this record was left unchanged.".into()));
    }
    if envelope.checksum != envelope.checksum()? {
        return Err(Error(
            "Save checksum mismatch. This record was left unchanged.".into(),
        ));
    }
    let introductory = envelope.engine == INTRO_ENGINE || envelope.engine == OLD_INTRO_ENGINE;
    let (checkpoint, human, policy): (Checkpoint, Option<Player>, Option<Policy>) = if introductory
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Opponent {
            policy: Policy,
            identity: String,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Introductory {
            human: Player,
            #[serde(rename = "match")]
            checkpoint: Checkpoint,
            #[serde(default)]
            opponent: Option<Opponent>,
        }
        let saved: Introductory = serde_json::from_value(envelope.payload)?;
        let policy = if envelope.engine == OLD_INTRO_ENGINE && saved.opponent.is_none() {
            // Explicit migration: preserve the frozen opponent used by v1 saves.
            if saved.human == Player::Runner {
                Policy::CorpV0
            } else {
                Policy::RunnerV0
            }
        } else {
            let opponent = saved
                .opponent
                .ok_or_else(|| Error("Missing saved opponent policy.".into()))?;
            if opponent.policy.role() != saved.human.other()
                || opponent.identity != opponent.policy.identity()
            {
                return Err(Error(
                    "Unsupported or changed saved opponent policy.".into(),
                ));
            }
            opponent.policy
        };
        (saved.checkpoint, Some(saved.human), Some(policy))
    } else {
        (serde_json::from_value(envelope.payload)?, None, None)
    };
    let game = Match::resume(checkpoint).map_err(|reason| Error(reason.into()))?;
    if game.is_introductory() != introductory {
        return Err(Error("Save engine does not match the game payload.".into()));
    }
    Ok((game, human, policy))
}
fn read(path: &Path) -> Result<Vec<u8>, Error> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(Error(
            "The save must be a regular file, not a link or directory.".into(),
        ));
    }
    record_replacement::read(path, LIMIT, Error)
}

fn location(path: &Path) -> Result<PathBuf, Error> {
    let absolute = std::path::absolute(path)?;
    let parent = absolute
        .parent()
        .ok_or_else(|| Error("A save path needs a file name.".into()))?;
    match parent.canonicalize() {
        Ok(parent) => Ok(parent.join(
            absolute
                .file_name()
                .ok_or_else(|| Error("A save path needs a file name.".into()))?,
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(absolute),
        Err(error) => Err(error.into()),
    }
}

pub(crate) struct Record {
    path: PathBuf,
    baseline: Option<Vec<u8>>,
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
            .commit_inner(game, true)
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
        let mut record = Self {
            path: options.path.clone(),
            baseline: None,
            human,
            policy,
        };
        let require_introductory = fresh.as_ref().map(Match::is_introductory);
        let game = if let Some(source) = &options.recover_from {
            let previous = record_replacement::previous_path(source);
            // Existing destinations (including aliases to source records) are
            // refused again under the shared writer lock when creating below.
            if location(source)? == location(&options.path)?
                || location(&previous)? == location(&options.path)?
            {
                return Err(Error("Recovery needs a fresh destination distinct from the active and previous records.".into()));
            }
            let (game, saved_human, saved_policy) = decode(&read(&previous)?)?;
            record.human = saved_human;
            record.policy = saved_policy;
            game
        } else {
            match fs::symlink_metadata(&options.path) {
                Ok(_) => {
                    let bytes = read(&options.path)?;
                    let (game, saved_human, saved_policy) = decode(&bytes)?;
                    if require_turns != game.has_turns() || require_introductory.is_some_and(|required| required != game.is_introductory()) { return Err(Error("The save requires different game content than this launch.".into())); }
                    record.human = saved_human;
                    record.policy = saved_policy;
                    record.baseline = Some(bytes);
                    return Ok((record, game));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => fresh.ok_or_else(|| Error("An existing ordinary-turn engineering save is required. No game was created.".into()))?,
                Err(error) => return Err(error.into()),
            }
        };
        if require_turns != game.has_turns()
            || require_introductory.is_some_and(|required| required != game.is_introductory())
        {
            return Err(Error(
                "Recovery requires the same game content as this launch.".into(),
            ));
        }
        record.commit_inner(&game, false)?;
        Ok((record, game))
    }
    pub fn commit(&mut self, game: &Match) -> Result<(), String> {
        self.commit_inner(game, false).map(|_| ()).map_err(|error| format!("Could not save: {error} Play stopped at the last committed decision. Close and reopen after repair."))
    }
    fn commit_inner(&mut self, game: &Match, archive: bool) -> Result<Option<PathBuf>, Error> {
        let bytes = encode(game, self.human, self.policy)?;
        let archived = record_replacement::replace(
            &self.path,
            &bytes,
            LIMIT,
            Error,
            |existing| {
                match (&self.baseline, existing) {
                (None, None) => match fs::symlink_metadata(record_replacement::previous_path(&self.path)) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Replacement::Create),
                    Err(error) => Err(error.into()),
                    Ok(_) => Err(Error("A previous record exists without an active save. Recover it explicitly into a fresh path.".into())),
                },
                (Some(expected), Some(existing)) if expected == existing => {
                    decode(existing)?;
                    Ok(if archive { Replacement::Archive } else if existing == bytes { Replacement::Unchanged } else { Replacement::Replace })
                }
                _ => Err(Error("The save changed, disappeared, or its destination is already occupied. Reopen it before continuing.".into())),
            }
            },
            |previous| decode(&read(previous)?).map(|_| ()),
        )?;
        self.baseline = Some(bytes);
        Ok(archived)
    }
}
