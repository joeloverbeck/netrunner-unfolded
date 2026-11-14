//! Native controller: the table receives observations, never the hidden match.
pub use crate::saved_state::SaveOptions;
use crate::{
    game::{Action, Match, Observation, Player, fixed_runner_response},
    saved_state::Record,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Admission {
    #[default]
    Loading,
    Ready,
    Refused(String),
}

pub struct Play {
    admission: Admission,
    game: Option<Match>,
    save: Option<SaveOptions>,
    record: Option<Record>,
    credits: u8,
    persistence_failure: Option<String>,
}
impl Default for Play {
    fn default() -> Self {
        Self::new(3, None)
    }
}
impl Play {
    pub fn new(credits: u8, save: Option<SaveOptions>) -> Self {
        Self {
            admission: Admission::Loading,
            game: None,
            save,
            record: None,
            credits,
            persistence_failure: None,
        }
    }
    pub fn saved(path: impl Into<std::path::PathBuf>) -> Self {
        Self::new(
            3,
            Some(SaveOptions {
                path: path.into(),
                recover_from: None,
            }),
        )
    }
    pub fn admission(&self) -> &Admission {
        &self.admission
    }
    pub fn observation(&self) -> Option<Observation> {
        self.observe(Player::Corp)
    }
    pub fn observe(&self, player: Player) -> Option<Observation> {
        self.game.as_ref().map(|game| game.observe(player))
    }
    pub fn admit(&mut self) {
        if self.admission != Admission::Loading {
            return;
        }
        let fresh = Match::prepared_bran_run(self.credits);
        let game = if let Some(options) = &self.save {
            match Record::open(options, fresh) {
                Ok((record, game)) => {
                    self.record = Some(record);
                    game
                }
                Err(reason) => {
                    self.refuse(reason);
                    return;
                }
            }
        } else {
            fresh
        };
        self.game = Some(game);
        self.admission = Admission::Ready;
    }
    pub fn refuse(&mut self, reason: String) {
        self.admission = Admission::Refused(reason);
    }
    fn candidate(&self) -> Result<Match, String> {
        if self.admission != Admission::Ready {
            return Err("Artwork or save is not admitted. Play is stopped.".into());
        }
        if let Some(error) = &self.persistence_failure {
            return Err(error.clone());
        }
        Match::resume(
            self.game
                .as_ref()
                .ok_or("The match has not started.")?
                .checkpoint(),
        )
        .map_err(String::from)
    }
    fn commit(&mut self, candidate: Match) -> Result<(), String> {
        if let Some(record) = &mut self.record
            && let Err(error) = record.commit(&candidate)
        {
            self.persistence_failure = Some(error.clone());
            return Err(error);
        }
        self.game = Some(candidate);
        Ok(())
    }
    pub fn choose(&mut self, revision: u64, action: Action) -> Result<(), String> {
        self.choose_for(Player::Corp, revision, action)
    }
    pub fn choose_for(
        &mut self,
        actor: Player,
        revision: u64,
        action: Action,
    ) -> Result<(), String> {
        let mut candidate = self.candidate()?;
        candidate.choose(actor, revision, action)?;
        self.commit(candidate)
    }
    pub fn respond(&mut self) -> Result<(), String> {
        let mut candidate = self.candidate()?;
        let view = candidate.observe(Player::Runner);
        if let Some(action) = fixed_runner_response(&view) {
            candidate.choose(
                Player::Runner,
                view.decision.as_ref().expect("legal response").revision,
                action,
            )?;
            self.commit(candidate)?;
        }
        Ok(())
    }
    pub fn restart(&mut self) -> Result<(), String> {
        let mut candidate = self.candidate()?;
        if candidate.observe(Player::Corp).run_result.is_none() {
            return Err("Finish the current run first.".into());
        }
        candidate.restart_position()?;
        self.commit(candidate)
    }
}
