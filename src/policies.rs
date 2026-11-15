//! Role policies receive only contemporaneous player observations. Neither
//! checkpoints, match seeds nor diagnostic records belong to this interface.
mod frozen;
use crate::game::{Action, Observation, Player};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Policy {
    CorpV0,
    RunnerV0,
}

impl Policy {
    pub fn role(self) -> Player {
        match self {
            Self::CorpV0 => Player::Corp,
            Self::RunnerV0 => Player::Runner,
        }
    }

    pub fn identity(self) -> String {
        format!(
            "{self:?}:sha256:{:x}",
            Sha256::digest(include_bytes!("policies/frozen.rs"))
        )
    }

    /// V0 is deterministic and stateless: no policy seed or memory is consumed.
    pub fn choose(self, observation: &Observation) -> Option<Action> {
        let decision = observation.decision.as_ref()?;
        if decision.actor != self.role() {
            return None;
        }
        frozen::response(observation)
    }
}
