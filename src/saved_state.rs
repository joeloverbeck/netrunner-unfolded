//! Netrunner admission policy around the shared locked replacement lifecycle.
use crate::game::{Checkpoint, Match};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};
use unfolded_tabletop::record_replacement::{self, Replacement};

const LIMIT: u64 = 1024 * 1024;
const ENGINE: &str = "bran-continuation-1";
const CONTENT: &str = include_str!("../resources/bran-assets.json");
const TURN_ENGINE: &str = "ordinary-turns-1";
const TURN_CONTENT: &str =
    "CR26.03;cards6ff880724741ea6ac062acf77e0cf979bb44e67e;turns-12;splitmix64-v1";

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
fn encode(game: &Match) -> Result<Vec<u8>, Error> {
    let checkpoint = game.checkpoint();
    Match::resume(checkpoint.clone()).map_err(|reason| Error(reason.into()))?;
    let mut envelope = Envelope {
        format: 1,
        kind: "netrunner-match".into(),
        engine: if game.has_turns() {
            TURN_ENGINE
        } else {
            ENGINE
        }
        .into(),
        content: digest(
            if game.has_turns() {
                TURN_CONTENT
            } else {
                CONTENT
            }
            .as_bytes(),
        ),
        payload: serde_json::to_value(checkpoint)?,
        checksum: String::new(),
    };
    envelope.checksum = envelope.checksum()?;
    Ok(serde_json::to_vec(&envelope)?)
}
fn decode(bytes: &[u8]) -> Result<Match, Error> {
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.format != 1
        || envelope.kind != "netrunner-match"
        || !((envelope.engine == ENGINE && envelope.content == digest(CONTENT.as_bytes()))
            || (envelope.engine == TURN_ENGINE
                && envelope.content == digest(TURN_CONTENT.as_bytes())))
    {
        return Err(Error("Unsupported save format, engine or required content. Use a compatible build and artwork; this record was left unchanged.".into()));
    }
    if envelope.checksum != envelope.checksum()? {
        return Err(Error(
            "Save checksum mismatch. This record was left unchanged.".into(),
        ));
    }
    let checkpoint: Checkpoint = serde_json::from_value(envelope.payload)?;
    let game = Match::resume(checkpoint).map_err(|reason| Error(reason.into()))?;
    if game.has_turns() != (envelope.engine == TURN_ENGINE) {
        return Err(Error("Save engine does not match the game payload.".into()));
    }
    Ok(game)
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
}
impl Record {
    pub fn open(options: &SaveOptions, fresh: Match) -> Result<(Self, Match), String> {
        Self::open_inner(options, Some(fresh), false).map_err(|error| format!("{error} To recover a valid previous record, use --recover-from ACTIVE --save FRESH. Close and reopen after repair."))
    }
    pub fn open_turns(options: &SaveOptions) -> Result<(Self, Match), String> {
        Self::open_inner(options, None, true).map_err(|error| error.to_string())
    }
    fn open_inner(
        options: &SaveOptions,
        fresh: Option<Match>,
        require_turns: bool,
    ) -> Result<(Self, Match), Error> {
        let mut record = Self {
            path: options.path.clone(),
            baseline: None,
        };
        let game = if let Some(source) = &options.recover_from {
            let previous = record_replacement::previous_path(source);
            // Existing destinations (including aliases to source records) are
            // refused again under the shared writer lock when creating below.
            if location(source)? == location(&options.path)?
                || location(&previous)? == location(&options.path)?
            {
                return Err(Error("Recovery needs a fresh destination distinct from the active and previous records.".into()));
            }
            decode(&read(&previous)?)?
        } else {
            match fs::symlink_metadata(&options.path) {
                Ok(_) => {
                    let bytes = read(&options.path)?;
                    let game = decode(&bytes)?;
                    if require_turns && !game.has_turns() { return Err(Error("This launch requires an existing ordinary-turn engineering save.".into())); }
                    record.baseline = Some(bytes);
                    return Ok((record, game));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => fresh.ok_or_else(|| Error("An existing ordinary-turn engineering save is required. No game was created.".into()))?,
                Err(error) => return Err(error.into()),
            }
        };
        if require_turns && !game.has_turns() {
            return Err(Error(
                "Recovery requires an ordinary-turn engineering save.".into(),
            ));
        }
        record.commit_inner(&game)?;
        Ok((record, game))
    }
    pub fn commit(&mut self, game: &Match) -> Result<(), String> {
        self.commit_inner(game).map_err(|error| format!("Could not save: {error} Play stopped at the last committed decision. Close and reopen after repair."))
    }
    fn commit_inner(&mut self, game: &Match) -> Result<(), Error> {
        let bytes = encode(game)?;
        record_replacement::replace(
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
                    Ok(if existing == bytes { Replacement::Unchanged } else { Replacement::Replace })
                }
                _ => Err(Error("The save changed, disappeared, or its destination is already occupied. Reopen it before continuing.".into())),
            }
            },
            |previous| decode(&read(previous)?).map(|_| ()),
        )?;
        self.baseline = Some(bytes);
        Ok(())
    }
}
