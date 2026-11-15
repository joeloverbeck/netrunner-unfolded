//! Role policies receive only contemporaneous player observations. Neither
//! checkpoints, match seeds nor diagnostic records belong to this interface.
mod corp;
mod corp_v1;
mod frozen;
mod runner;
use crate::game::{Action, Observation, Player};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Policy {
    CorpV0,
    CorpV1,
    CorpV2,
    RunnerV0,
    RunnerV1,
}

impl Policy {
    pub fn role(self) -> Player {
        match self {
            Self::CorpV0 | Self::CorpV1 | Self::CorpV2 => Player::Corp,
            Self::RunnerV0 | Self::RunnerV1 => Player::Runner,
        }
    }

    pub fn identity(self) -> String {
        if self == Self::RunnerV1 {
            let mut source = Sha256::new();
            source.update(include_bytes!("policies/runner.rs"));
            source.update(include_bytes!("game/mechanics.rs"));
            return format!("{self:?}:sha256:{:x}", source.finalize());
        }
        format!(
            "{self:?}:sha256:{:x}",
            Sha256::digest(match self {
                Self::CorpV1 => include_bytes!("policies/corp_v1.rs").as_slice(),
                Self::CorpV2 => include_bytes!("policies/corp.rs").as_slice(),
                _ => include_bytes!("policies/frozen.rs").as_slice(),
            })
        )
    }

    /// All admitted policies are deterministic and stateless.
    pub fn choose(self, observation: &Observation) -> Option<Action> {
        let decision = observation.decision.as_ref()?;
        if decision.actor != self.role() {
            return None;
        }
        match self {
            Self::CorpV1 => corp_v1::response(observation),
            Self::CorpV2 => corp::response(observation),
            Self::RunnerV1 => runner::response(observation),
            _ => frozen::response(observation),
        }
    }
}
