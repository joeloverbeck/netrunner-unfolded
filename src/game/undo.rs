//! One-choice correction, separate from legal game actions and bot policy.
use super::*;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Correction {
    before: Option<State>,
    actor: Option<Player>,
    information: bool,
}

impl Correction {
    pub(super) fn valid(&self, current: &State) -> bool {
        self.before.as_ref().is_none_or(|before| {
            self.actor.is_some()
                && before
                    .pending
                    .is_some_and(|pending| Some(pending.actor()) == self.actor)
                && !self.information
                && before.valid()
                && before.revision < current.revision
                && before.turn.is_some() == current.turn.is_some()
                && before.turn.as_ref().map(|turn| turn.introductory)
                    == current.turn.as_ref().map(|turn| turn.introductory)
                && !information_changed(before, current)
        })
    }
}

fn information_changed(before: &State, after: &State) -> bool {
    before.information_revision != after.information_revision
        || before.turn.as_ref().map(|t| t.random_state)
            != after.turn.as_ref().map(|t| t.random_state)
        || after.cards.iter().any(|card| {
            before
                .cards
                .iter()
                .find(|old| old.id == card.id)
                .is_none_or(|old| {
                    [Player::Corp, Player::Runner].into_iter().any(|viewer| {
                        let visible = |state: &State, card: &CardInstance| {
                            if state.turn.is_some() {
                                state.turn_view(card, viewer).printing.is_some()
                            } else {
                                card.view(viewer).printing.is_some()
                            }
                        };
                        visible(after, card) && !visible(before, old)
                    })
                })
        })
}

impl Match {
    /// No private card identity or internal chance state enters this explanation.
    pub fn undo_reason(&self, viewer: Player) -> Option<&'static str> {
        if self.1.information {
            Some("Undo unavailable: new information or a random outcome was committed.")
        } else if self.1.actor.is_some_and(|actor| actor != viewer) {
            Some("Undo unavailable: the opponent has responded.")
        } else if self.1.before.is_none() {
            Some("Undo unavailable: no reversible choice. Only the latest choice can be corrected.")
        } else {
            None
        }
    }

    pub fn undo(&mut self, actor: Player, revision: u64) -> Result<(), &'static str> {
        if revision != self.0.revision {
            return Err("This correction is stale.");
        }
        if let Some(reason) = self.undo_reason(actor) {
            return Err(reason);
        }
        let next = self
            .0
            .revision
            .checked_add(1)
            .ok_or("Decision revision exhausted")?;
        let mut before = self.1.before.clone().expect("available correction");
        before.revision = next;
        if !before.valid() {
            return Err("Decision revision exhausted. The committed game is unchanged.");
        }
        self.0 = before;
        self.1 = Correction::default();
        Ok(())
    }

    pub(super) fn record_correction(&mut self, before: State, actor: Player, forced: bool) {
        if information_changed(&before, &self.0) {
            self.1 = Correction {
                information: true,
                ..Default::default()
            };
        } else if !forced {
            self.1 = Correction {
                before: Some(before),
                actor: Some(actor),
                information: false,
            };
        }
    }
}
