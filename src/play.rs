//! Native controller: the table receives observations, never the hidden match.
pub use crate::saved_state::SaveOptions;
mod opponent;
use crate::{
    game::{Action, EngineeringPosition, Match, Observation, Player},
    policies::Policy,
    saved_state::Record,
};
pub use opponent::{
    OPPONENT_BUDGET, OpponentOutcome, OpponentProposal, OpponentReport, OpponentRequest,
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
    introductory_seed: Option<u64>,
    role_selection: bool,
    policy: Policy,
    opponent_epoch: u64,
    opponent_cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    opponent_report: Option<OpponentReport>,
    opponent_worker: Option<opponent::Worker>,
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
            introductory_seed: None,
            role_selection: false,
            policy: Policy::RunnerV0,
            opponent_epoch: 0,
            opponent_cancellation: Default::default(),
            opponent_report: None,
            opponent_worker: None,
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
    /// Freshness without disclosing the opponent's choices or hidden state.
    pub fn revision(&self) -> Option<u64> {
        self.game.as_ref().map(Match::revision)
    }
    /// Whether the native opponent has a response to make through `respond`.
    pub fn opponent_pending(&self) -> bool {
        self.observe(self.human.other()).is_some_and(|view| {
            view.decision
                .is_some_and(|d| d.actor == self.human.other() && !d.choices.is_empty())
        })
    }
    pub fn with_seat(mut self, human: Player) -> Self {
        self.human = human;
        self.policy = if human == Player::Corp {
            Policy::RunnerV0
        } else {
            Policy::CorpV0
        };
        self
    }
    /// Ordinary setup with an explicit seed for deterministic engineering checks.
    /// The native launcher supplies fresh independent entropy.
    pub fn introductory_seeded(seed: u64, save: Option<SaveOptions>, human: Player) -> Self {
        let mut play = Self::new(5, save).with_seat(human);
        play.introductory_seed = Some(seed);
        play.policy = Self::introductory_policy(human);
        play
    }
    /// Normal launch: independent chance and a role choice unless one was
    /// explicitly supplied. A saved seat and chance always take precedence.
    pub fn introductory(save: Option<SaveOptions>, human: Option<Player>) -> Result<Self, String> {
        let seed = unfolded_tabletop::random::fresh_seed()
            .map_err(|error| format!("Could not prepare independent match chance: {error}"))?;
        let play = Self::introductory_seeded(seed, save, human.unwrap_or(Player::Corp));
        Ok(if human.is_some() {
            play
        } else {
            play.with_role_selection()
        })
    }
    pub fn with_role_selection(mut self) -> Self {
        self.role_selection = self.introductory_seed.is_some();
        self
    }
    pub fn needs_seat(&self) -> bool {
        self.role_selection && self.admission == Admission::Ready
    }
    #[cfg(feature = "tabletop")]
    pub(crate) fn uses_turn_table(&self) -> bool {
        self.introductory_seed.is_some() || self.game.as_ref().is_some_and(Match::has_turns)
    }
    pub fn begin_new_match_seeded(&mut self, seed: u64) -> Result<(), String> {
        if !self.can_start_new_match() {
            return Err("A new match is available after this introductory match ends.".into());
        }
        self.introductory_seed = Some(seed);
        self.role_selection = true;
        Ok(())
    }
    pub fn begin_new_match(&mut self) -> Result<(), String> {
        if !self.can_start_new_match() {
            return Err("A new match is available after this introductory match ends.".into());
        }
        let seed = unfolded_tabletop::random::fresh_seed()
            .map_err(|error| format!("Could not prepare independent match chance: {error}"))?;
        self.begin_new_match_seeded(seed)
    }
    pub fn can_start_new_match(&self) -> bool {
        self.admission == Admission::Ready
            && self.persistence_failure.is_none()
            && !self.role_selection
            && self.game.as_ref().is_some_and(|game| {
                game.is_introductory() && game.observe(self.human).match_winner.is_some()
            })
    }
    pub fn select_seat(&mut self, human: Player) -> Result<(), String> {
        if !self.needs_seat() {
            return Err("Choose a role only before a new match begins.".into());
        }
        if let Some(previous) = &self.game {
            let mut candidate =
                Match::introductory(self.introductory_seed.expect("introductory setup"));
            if let Err(error) = candidate.start_after(previous.revision()) {
                self.role_selection = false;
                return Err(error.into());
            }
            if let Some(record) = &mut self.record
                && let Err(error) =
                    record.commit_introductory(&candidate, human, Self::introductory_policy(human))
            {
                self.persistence_failure = Some(error.clone());
                self.role_selection = false;
                return Err(error);
            }
            self.human = human;
            self.policy = Self::introductory_policy(human);
            self.cancel_opponent();
            self.role_selection = false;
            self.game = Some(candidate);
            return Ok(());
        }
        self.human = human;
        self.policy = Self::introductory_policy(human);
        self.role_selection = false;
        self.admission = Admission::Loading;
        self.admit();
        match &self.admission {
            Admission::Ready => Ok(()),
            Admission::Refused(reason) => Err(reason.clone()),
            Admission::Loading => Err("The match has not started.".into()),
        }
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
        if self.role_selection {
            return None;
        }
        self.game.as_ref().map(|game| game.observe(player))
    }
    pub fn admit(&mut self) {
        if self.admission != Admission::Loading {
            return;
        }
        if self.role_selection
            && self
                .save
                .as_ref()
                .is_none_or(|options| options.recover_from.is_none() && !options.path.exists())
        {
            self.admission = Admission::Ready;
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
            None => match self.introductory_seed {
                Some(seed) => Match::introductory(seed),
                None => Match::prepared_bran_run(self.credits),
            },
        };
        let game = if let Some(options) = &self.save {
            match if self.introductory_seed.is_some() {
                Record::open_introductory(options, fresh, self.human, self.policy)
            } else if self.require_turns {
                Record::open_turns(options)
            } else {
                Record::open(options, fresh)
            } {
                Ok((record, game)) => {
                    if let Some(human) = record.human() {
                        self.human = human;
                        self.policy = record.policy().expect("introductory opponent");
                    }
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
        self.role_selection = false;
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
        self.cancel_opponent();
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
        self.cancel_opponent();
        Ok(())
    }
    pub fn choose(&mut self, revision: u64, action: Action) -> Result<(), String> {
        self.choose_for(self.human, revision, action)
    }
    pub fn undo_reason(&self) -> Option<&'static str> {
        self.game
            .as_ref()
            .and_then(|game| game.undo_reason(self.human))
    }
    pub fn undo(&mut self, revision: u64) -> Result<(), String> {
        let mut candidate = self.candidate()?;
        candidate.undo(self.human, revision)?;
        self.commit(candidate)
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
        if let Some(request) = self.prepare_opponent(OPPONENT_BUDGET)? {
            self.commit_opponent(request.compute())?;
        }
        Ok(())
    }
    fn introductory_policy(human: Player) -> Policy {
        if human == Player::Runner {
            Policy::CorpV1
        } else {
            Policy::RunnerV0
        }
    }
    pub fn restart(&mut self) -> Result<(), String> {
        let mut candidate = self.candidate()?;
        if candidate.has_turns() {
            return Err("A match cannot be reset or reshuffled. Reopen its save to continue, or choose a new save for a new game.".into());
        }
        if candidate.observe(Player::Corp).run_result.is_none() {
            return Err("Finish the current run first.".into());
        }
        candidate.restart_position()?;
        self.commit(candidate)
    }
}
