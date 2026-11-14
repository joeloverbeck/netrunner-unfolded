//! Bounded Brân/Palisade position from issue #1, using CR 26.03.
//! This is the first match implementation, not a complete-game rules claim.

mod state;
mod turns;
pub use turns::{EngineeringCard, EngineeringPosition, Server, ServerView, TurnWindow};
use serde::{Deserialize, Serialize};
pub use state::{CardId, CardKind, Checkpoint, Zone};
use state::{CardInstance, IcePosition, Pending, PositionId, Run, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Player {
    Corp,
    Runner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Action {
    DeclineInstall,
    InstallPalisade,
    RetainBran,
    TrashBran,
    Continue,
    RezPalisade,
    DeclineRez,
    JackOut,
    PassPriority,
    GainCredit,
    Draw,
    Purge,
    PlayCard(CardId),
    InstallCard(CardId),
    Destination(Server),
    TrashInstalled(CardId),
    FinishInstall,
    Advance(CardId),
    Score(CardId),
    Rez(CardId),
    SelectTarget(CardId),
    ToggleDiscard(CardId),
    ConfirmDiscard,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::DeclineInstall => "Decline installation",
            Self::InstallPalisade => "Install Palisade · free",
            Self::RetainBran => "Keep Brân",
            Self::TrashBran => "Trash Brân",
            Self::Continue => "Continue run",
            Self::RezPalisade => "Rez Palisade · 3 credits",
            Self::DeclineRez => "Leave unrezzed",
            Self::JackOut => "Jack out",
            Self::PassPriority => "Pass priority",
            Self::GainCredit => "Gain 1 credit · 1 click",
            Self::Draw => "Draw 1 card · 1 click",
            Self::Purge => "Purge virus counters · 3 clicks",
            Self::PlayCard(_) => "Play card · 1 click + card cost",
            Self::InstallCard(_) => "Install card · 1 click + install cost",
            Self::Destination(_) => "Choose destination",
            Self::TrashInstalled(_) => "Trash installed card",
            Self::FinishInstall => "Finish installation",
            Self::Advance(_) => "Advance · 1 click + 1 credit",
            Self::Score(_) => "Score agenda",
            Self::Rez(_) => "Rez card",
            Self::SelectTarget(_) => "Place 2 advancement counters here",
            Self::ToggleDiscard(_) => "Select / deselect for discard",
            Self::ConfirmDiscard => "Discard selected cards",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub actor: Player,
    pub revision: u64,
    pub choices: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunResult {
    EndedByIce,
    SuccessfulEmptyHq,
    JackedOut,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CardView {
    pub instance: Option<CardId>,
    pub title: Option<String>,
    pub rezzed: bool,
    pub printing: Option<&'static str>,
    pub rules_text: Option<&'static str>,
    pub advancement: u32,
    pub selected: bool,
}

const BRAN_TEXT: &str = "Lose 1 click: Break 1 subroutine on this ice. Only the Runner can use this ability.\nSubroutine: You may install 1 piece of ice from HQ or Archives directly inward from this ice, ignoring all costs.\nSubroutine: End the run.\nSubroutine: End the run.";
const PALISADE_TEXT: &str = "While this ice is protecting a remote server, it gets +2 strength.\nSubroutine: End the run.\nRez: 3 credits. Strength at HQ: 2.";

impl CardView {
    fn face(
        title: &str,
        printing: &'static str,
        text: &'static str,
        visible: bool,
        rezzed: bool,
    ) -> Self {
        Self {
            instance: None,
            title: visible.then(|| title.into()),
            printing: visible.then_some(printing),
            rules_text: visible.then_some(text),
            rezzed,
            advancement: 0,
            selected: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Observation {
    pub corp_identity: CardView,
    pub runner_identity: CardView,
    pub decision: Option<Decision>,
    pub corp_credits: u32,
    pub runner_credits: u32,
    pub clicks: [u8; 2],
    pub agenda_points: [u8; 2],
    pub active_player: Option<Player>,
    pub turn_number: u64,
    pub rd_count: usize,
    pub stack_count: usize,
    pub grip_count: usize,
    pub rig: Vec<CardView>,
    pub heap: Vec<CardView>,
    pub corp_score: Vec<CardView>,
    pub runner_score: Vec<CardView>,
    pub servers: Vec<ServerView>,
    pub memory_used: u8,
    pub terminal_reason: Option<&'static str>,
    pub run_result: Option<RunResult>,
    pub match_winner: Option<Player>,
    pub hq_count: usize,
    /// Innermost first, independent of the current run position.
    pub ice: Vec<CardView>,
    pub archives: Vec<CardView>,
    pub approached_ice: Option<usize>,
    pub hand: Vec<CardView>,
    pub installing: Option<CardView>,
    pub history: Vec<String>,
    pub phase: &'static str,
}

pub struct Match(State);

impl Match {
    /// Capture a suspended decision without resolving it or replaying actions.
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            version: if self.0.turn.is_some() { 2 } else { 1 },
            rules: state::RULES.into(),
            card_revision: state::CARD_REVISION.into(),
            state: self.0.clone(),
        }
    }

    /// Restore only a supported, coherent game payload. Player observations
    /// remain the only interface to pass to a human seat or opponent policy.
    pub fn resume(checkpoint: Checkpoint) -> Result<Self, &'static str> {
        if checkpoint.version != if checkpoint.state.turn.is_some() { 2 } else { 1 }
            || checkpoint.rules != state::RULES
            || checkpoint.card_revision != state::CARD_REVISION
        {
            return Err("Unsupported match continuation.");
        }
        if !checkpoint.state.valid() {
            return Err("Invalid match continuation.");
        }
        Ok(Self(checkpoint.state))
    }

    pub(crate) fn restart_position(&mut self) -> Result<(), &'static str> {
        let revision = self
            .0
            .revision
            .checked_add(1)
            .ok_or("Decision revision exhausted. Use a fresh save path.")?;
        let mut candidate = Self::prepared_bran_run(3);
        candidate.0.revision = revision;
        if !candidate.0.valid() {
            return Err("Decision revision exhausted. Use a fresh save path.");
        }
        *self = candidate;
        Ok(())
    }
    pub fn prepared_bran_run(corp_credits: u8) -> Self {
        let bran = CardId(1);
        let palisade = CardId(2);
        let position = PositionId(1);
        Self(State { corp_credits: corp_credits.into(), revision: 0, run_result: None, turn: None,
            cards: vec![
                CardInstance::new(bran, CardKind::Bran, Zone::ProtectingHq, true),
                CardInstance::new(palisade, CardKind::Palisade, Zone::Hq, false),
            ],
            hq_positions: vec![IcePosition { id: position, card: Some(bran) }],
            run: Some(Run { position, encountered: Some(bran) }),
            pending: Some(Pending::BranInstall { source: bran, card: palisade }),
            history: vec![["Brân's first subroutine resolves. The Runner has no clicks or applicable paid abilities.".into(), "Brân's first subroutine resolves. The Runner has no clicks or applicable paid abilities.".into()]] })
    }
    pub fn observe(&self, viewer: Player) -> Observation {
        if self.0.turn.is_some() { return self.observe_turns(viewer); }
        let state = &self.0;
        let ice = state
            .hq_positions
            .iter()
            .filter_map(|position| position.card)
            .map(|id| state.card(id).view(viewer))
            .collect();
        let decision = state
            .pending
            .filter(|pending| pending.actor() == viewer)
            .map(|pending| Decision {
                actor: pending.actor(),
                revision: state.revision,
                choices: match pending {
                    Pending::BranInstall { .. } => {
                        vec![Action::DeclineInstall, Action::InstallPalisade]
                    }
                    Pending::BranInstalling { .. } => vec![Action::RetainBran, Action::TrashBran],
                    Pending::Movement => vec![Action::Continue, Action::JackOut],
                    Pending::Rez { .. } if state.corp_credits >= 3 => {
                        vec![Action::RezPalisade, Action::DeclineRez]
                    }
                    Pending::Rez { .. } => vec![Action::DeclineRez],
                    _ => unreachable!("ordinary turn pending"),
                },
            });
        Observation {
            corp_identity: CardView::face(
                "The Syndicate",
                "30077",
                "Starter game only.",
                true,
                false,
            ),
            runner_identity: CardView::face(
                "The Catalyst",
                "30076",
                "Starter game only.",
                true,
                false,
            ),
            decision,
            corp_credits: state.corp_credits,
            runner_credits: 0, clicks: [0, 0], agenda_points: [0, 0], active_player: None,
            turn_number: 0, rd_count: 0, stack_count: 0, grip_count: 0, rig: vec![], heap: vec![],
            corp_score: vec![], runner_score: vec![], servers: vec![], memory_used: 0, terminal_reason: None,
            run_result: state.run_result,
            match_winner: None,
            hq_count: state
                .cards
                .iter()
                .filter(|card| card.zone == Zone::Hq)
                .count(),
            ice,
            archives: state
                .cards
                .iter()
                .filter(|card| card.zone == Zone::Archives)
                .map(|card| card.view(viewer))
                .collect(),
            approached_ice: match state.pending {
                Some(Pending::Rez { ice }) => state
                    .hq_positions
                    .iter()
                    .filter_map(|position| position.card)
                    .position(|id| id == ice),
                _ => None,
            },
            hand: state
                .cards
                .iter()
                .filter(|card| card.zone == Zone::Hq)
                .map(|card| card.view(viewer))
                .collect(),
            installing: state
                .cards
                .iter()
                .find(|card| card.zone == Zone::Installing)
                .map(|card| card.view(viewer)),
            history: state
                .history
                .iter()
                .map(|event| event[usize::from(viewer == Player::Runner)].clone())
                .collect(),
            phase: match state.pending {
                Some(Pending::BranInstall { .. }) => "Brân: optional installation",
                Some(Pending::BranInstalling { .. }) => "Installing inward: keep or trash Brân",
                Some(Pending::Movement) if state.current_position() > 0 => {
                    "Encounter ended · Runner may jack out"
                }
                Some(Pending::Rez { .. }) => "Approaching the new ICE · Corp may rez",
                Some(Pending::Movement) => "Unrezzed ICE passed · Runner may jack out",
                None => "Run complete · match unfinished",
                _ => unreachable!("ordinary turn pending"),
            },
        }
    }
    pub fn choose(
        &mut self,
        actor: Player,
        revision: u64,
        action: Action,
    ) -> Result<(), &'static str> {
        let decision = self
            .observe(actor)
            .decision
            .ok_or("No decision for this player.")?;
        if revision != decision.revision || !decision.choices.contains(&action) {
            return Err("This choice is stale or unavailable.");
        }
        if self.0.turn.is_some() { return self.choose_turns(actor, action); }
        let state = &mut self.0;
        let next_revision = state
            .revision
            .checked_add(1)
            .ok_or("Match revision exhausted.")?;
        let install_position = if action == Action::InstallPalisade {
            Some(
                state
                    .next_position_id()
                    .ok_or("Run position identifiers exhausted.")?,
            )
        } else {
            None
        };
        let pending = state.pending.expect("validated decision");
        let (corp, runner) = match action {
            Action::DeclineInstall => (
                "Corp declines installation. Brân's next unbroken subroutine ends the run.",
                None,
            ),
            Action::InstallPalisade => (
                "Palisade enters the play area for installation inward from Brân. Installation costs are ignored.",
                Some(
                    "Corp starts installing a facedown card inward from Brân. Installation costs are ignored.",
                ),
            ),
            Action::RetainBran => (
                "Palisade is installed inward. Brân remains; its next unbroken subroutine ends the run.",
                Some(
                    "A facedown card is installed inward. Brân remains; its next unbroken subroutine ends the run.",
                ),
            ),
            Action::TrashBran => (
                "Brân is trashed faceup to Archives. Palisade's installation completes. Brân's encounter ends; its remaining subroutines do not resolve.",
                Some(
                    "Brân is trashed faceup to Archives. The installation completes. Brân's encounter ends; its remaining subroutines do not resolve.",
                ),
            ),
            Action::Continue if state.current_position() > 0 => (
                "The Runner continues and approaches Palisade in the new inward position.",
                Some(
                    "The Runner continues and approaches the facedown card in the new inward position.",
                ),
            ),
            Action::Continue => (
                "The Runner continues, successfully breaches empty HQ, accesses no cards, and ends the run. The match has not ended.",
                None,
            ),
            Action::RezPalisade => (
                "Corp pays 3 credits to rez Palisade. Its unbroken subroutine ends the run.",
                None,
            ),
            Action::DeclineRez => (
                "Corp declines to rez. The Runner passes the unrezzed ICE and may jack out before approaching HQ.",
                None,
            ),
            Action::JackOut => (
                "The Runner jacks out. The run ends unsuccessfully; the match has not ended.",
                None,
            ),
            _ => unreachable!("ordinary turn action"),
        };
        state
            .history
            .push([corp.into(), runner.unwrap_or(corp).into()]);
        match (pending, action) {
            (Pending::BranInstall { source, card }, Action::InstallPalisade) => {
                // CR 6.2.2 and 8.5.16b: declare the new position before the
                // optional trash. It is independent of Brân's physical card.
                let inward = state.current_position();
                let position = install_position.expect("validated installation position");
                state.hq_positions.insert(
                    inward,
                    IcePosition {
                        id: position,
                        card: None,
                    },
                );
                state.card_mut(card).zone = Zone::Installing;
                state.pending = Some(Pending::BranInstalling {
                    source,
                    card,
                    position,
                });
            }
            (Pending::BranInstall { .. }, Action::DeclineInstall) => {
                state.finish_run(RunResult::EndedByIce)
            }
            (
                Pending::BranInstalling {
                    source,
                    card,
                    position,
                },
                Action::RetainBran | Action::TrashBran,
            ) => {
                state.card_mut(card).zone = Zone::ProtectingHq;
                state
                    .hq_positions
                    .iter_mut()
                    .find(|place| place.id == position)
                    .expect("install position")
                    .card = Some(card);
                if action == Action::TrashBran {
                    let source_card = state.card_mut(source);
                    source_card.zone = Zone::Archives;
                    source_card.rezzed = false;
                    state
                        .hq_positions
                        .iter_mut()
                        .find(|place| place.card == Some(source))
                        .expect("encountered position")
                        .card = None;
                    // The Runner keeps this vacant position until movement.
                    state.run.as_mut().expect("active run").encountered = None;
                    state.pending = Some(Pending::Movement);
                } else {
                    state.finish_run(RunResult::EndedByIce);
                }
            }
            (Pending::Movement, Action::Continue) => {
                let current = state.current_position();
                if let Some(inward) = current.checked_sub(1) {
                    let next = &state.hq_positions[inward];
                    state.run.as_mut().expect("active run").position = next.id;
                    state.pending = Some(Pending::Rez {
                        ice: next.card.expect("installed ICE"),
                    });
                    if state.hq_positions[current].card.is_none() {
                        state.hq_positions.remove(current);
                    }
                } else {
                    state.finish_run(RunResult::SuccessfulEmptyHq);
                }
            }
            (Pending::Rez { .. }, Action::DeclineRez) => state.pending = Some(Pending::Movement),
            (Pending::Movement, Action::JackOut) => state.finish_run(RunResult::JackedOut),
            (Pending::Rez { ice }, Action::RezPalisade) => {
                state.corp_credits -= 3;
                state.card_mut(ice).rezzed = true;
                state.finish_run(RunResult::EndedByIce);
            }
            _ => unreachable!("validated legal choice"),
        }
        state.revision = next_revision;
        Ok(())
    }
}

/// The temporary opponent knows only its projection and submits a normal choice.
pub fn fixed_runner_response(observation: &Observation) -> Option<Action> {
    let decision = observation.decision.as_ref()?;
    (decision.actor == Player::Runner && decision.choices.contains(&Action::Continue))
        .then_some(Action::Continue)
}

/// Lawful temporary policy: only the acting player's projection enters here.
pub fn baseline_response(observation: &Observation) -> Option<Action> {
    if observation.active_player.is_none() { return fixed_runner_response(observation); }
    let decision = observation.decision.as_ref()?;
    decision.choices.first().copied()
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn changing_an_unobserved_face_does_not_change_runner_inputs() {
        let mut actual = Match::prepared_bran_run(3);
        let mut alternate = Match::prepared_bran_run(3);
        // Projection-only fixture: it admits no additional card effects.
        alternate.0.card_mut(CardId(2)).kind = CardKind::ConcealedFixture;
        assert_ne!(
            actual.observe(Player::Corp),
            alternate.observe(Player::Corp)
        );
        assert_eq!(
            actual.observe(Player::Runner),
            alternate.observe(Player::Runner)
        );
        assert_eq!(
            fixed_runner_response(&actual.observe(Player::Runner)),
            fixed_runner_response(&alternate.observe(Player::Runner))
        );
        for (actor, action) in [
            (Player::Corp, Action::InstallPalisade),
            (Player::Corp, Action::TrashBran),
            (Player::Runner, Action::Continue),
        ] {
            for game in [&mut actual, &mut alternate] {
                let revision = game.observe(actor).decision.unwrap().revision;
                game.choose(actor, revision, action).unwrap();
            }
            assert_ne!(
                actual.observe(Player::Corp),
                alternate.observe(Player::Corp)
            );
            assert_eq!(
                actual.observe(Player::Runner),
                alternate.observe(Player::Runner)
            );
        }
    }
}
