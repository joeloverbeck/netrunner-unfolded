//! Native controller: the table receives observations, never the hidden match.
use crate::game::{Action, Match, Observation, Player, fixed_runner_response};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Admission {
    #[default]
    Loading,
    Ready,
    Refused(String),
}

#[derive(Default)]
pub struct Play {
    admission: Admission,
    game: Option<Match>,
}

impl Play {
    pub fn admission(&self) -> &Admission {
        &self.admission
    }
    pub fn observation(&self) -> Option<Observation> {
        self.game.as_ref().map(|game| game.observe(Player::Corp))
    }
    pub fn admit(&mut self) {
        if self.admission == Admission::Loading {
            self.game = Some(Match::prepared_bran_run(3));
            self.admission = Admission::Ready;
        }
    }
    pub fn refuse(&mut self, reason: String) {
        self.admission = Admission::Refused(reason);
    }
    fn game_mut(&mut self) -> Result<&mut Match, &'static str> {
        if self.admission != Admission::Ready {
            return Err("Artwork is not admitted. Play is stopped.");
        }
        self.game.as_mut().ok_or("The match has not started.")
    }
    pub fn choose(&mut self, revision: u64, action: Action) -> Result<(), &'static str> {
        self.game_mut()?.choose(Player::Corp, revision, action)
    }
    pub fn respond(&mut self) -> Result<(), &'static str> {
        let game = self.game_mut()?;
        let view = game.observe(Player::Runner);
        if let Some(action) = fixed_runner_response(&view) {
            game.choose(
                Player::Runner,
                view.decision.as_ref().expect("legal response").revision,
                action,
            )?;
        }
        Ok(())
    }
    pub fn restart(&mut self) -> Result<(), &'static str> {
        let game = self.game_mut()?;
        if game.observe(Player::Corp).run_result.is_none() {
            return Err("Finish the current run first.");
        }
        game.restart_position();
        Ok(())
    }
}
