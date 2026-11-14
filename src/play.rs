//! Native controller: the table receives observations, never the hidden match.
pub use crate::saved_state::SaveOptions;
use crate::{
    game::{Action, EngineeringPosition, Match, Observation, Player, baseline_response},
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
    human: Player,
    engineering: Option<EngineeringPosition>,
    require_turns: bool,
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
            human: Player::Corp,
            engineering: None,
            require_turns: false,
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
        self.observe(self.human)
    }
    pub fn human(&self) -> Player {
        self.human
    }
    pub fn with_seat(mut self, human: Player) -> Self {
        self.human = human;
        self
    }
    pub fn load_turns(save: SaveOptions, human: Player) -> Self {
        let mut play = Self::new(3, Some(save)).with_seat(human);
        play.require_turns = true;
        play
    }
    /// Engineering driver only; the ordinary native launcher has no position preset.
    pub fn engineering(
        position: EngineeringPosition,
        save: Option<SaveOptions>,
        human: Player,
    ) -> Self {
        let mut play = Self::new(3, save);
        play.engineering = Some(position);
        play.human = human;
        play
    }
    pub fn observe(&self, player: Player) -> Option<Observation> {
        self.game.as_ref().map(|game| game.observe(player))
    }
    pub fn admit(&mut self) {
        if self.admission != Admission::Loading {
            return;
        }
        let fresh = match self.engineering.take() {
            Some(position) => match Match::engineering_turns(position) {
                Ok(game) => game,
                Err(error) => {
                    self.refuse(error.into());
                    return;
                }
            },
            None => Match::prepared_bran_run(self.credits),
        };
        let game = if let Some(options) = &self.save {
            match if self.require_turns {
                Record::open_turns(options)
            } else {
                Record::open(options, fresh)
            } {
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
        if self
            .game
            .as_ref()
            .is_some_and(|game| Self::forced_pass(game, self.human).is_some())
            && let Err(error) = self
                .candidate()
                .and_then(|candidate| self.commit(candidate))
        {
            self.refuse(error);
        }
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
    fn forced_pass(game: &Match, human: Player) -> Option<u64> {
        game.observe(human)
            .decision
            .filter(|decision| decision.choices.as_slice() == [Action::PassPriority])
            .map(|decision| decision.revision)
    }
    fn commit(&mut self, mut candidate: Match) -> Result<(), String> {
        // Only a paid window with no alternative can be skipped. Single target,
        // payment, discard, steal and installation choices remain explicit.
        // A turn boundary can expose the next turn's forced window as well.
        for _ in 0..8 {
            let Some(revision) = Self::forced_pass(&candidate, self.human) else {
                break;
            };
            candidate.choose(self.human, revision, Action::PassPriority)?;
        }
        if Self::forced_pass(&candidate, self.human).is_some() {
            return Err(
                "Automatic priority limit reached. The committed game is unchanged.".into(),
            );
        }
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
        self.choose_for(self.human, revision, action)
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
        let opponent = self.human.other();
        let view = candidate.observe(opponent);
        if let Some(action) = baseline_response(&view) {
            candidate.choose(
                opponent,
                view.decision.as_ref().expect("legal response").revision,
                action,
            )?;
            self.commit(candidate)?;
        }
        Ok(())
    }
    pub fn restart(&mut self) -> Result<(), String> {
        let mut candidate = self.candidate()?;
        if candidate.observe(self.human).active_player.is_some() {
            return Err("Engineering turn positions cannot restart from ordinary play.".into());
        }
        if candidate.observe(Player::Corp).run_result.is_none() {
            return Err("Finish the current run first.".into());
        }
        candidate.restart_position()?;
        self.commit(candidate)
    }
}
