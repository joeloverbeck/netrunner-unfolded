//! Role policies receive only contemporaneous player observations. Neither
//! checkpoints, match seeds nor diagnostic records belong to this interface.
mod corp;
mod frozen;
use crate::game::{Action, Observation, Player};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Policy {
    CorpV0,
    CorpV1,
    RunnerV0,
}

impl Policy {
    pub fn role(self) -> Player {
        match self {
            Self::CorpV0 | Self::CorpV1 => Player::Corp,
            Self::RunnerV0 => Player::Runner,
        }
    }

    pub fn identity(self) -> String {
        format!(
            "{self:?}:sha256:{:x}",
            Sha256::digest(match self {
                Self::CorpV1 => include_bytes!("policies/corp.rs").as_slice(),
                _ => include_bytes!("policies/frozen.rs").as_slice(),
            })
        )
    }

    /// V0 is deterministic and stateless: no policy seed or memory is consumed.
    pub fn choose(self, observation: &Observation) -> Option<Action> {
        let decision = observation.decision.as_ref()?;
        if decision.actor != self.role() {
            return None;
        }
        match self {
            Self::CorpV1 => corp::response(observation),
            _ => frozen::response(observation),
        }
    }
}
