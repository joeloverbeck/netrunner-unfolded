//! Netrunner-owned card locations and suspended run decisions. Position identity
//! follows CR 26.03 §§6.2.4–6.2.6, including a vacant position occupied by Runner.
use super::turns::{Server, Turn, TurnWindow};
use super::{BRAN_TEXT, CardView, PALISADE_TEXT, Player, RunResult};
use serde::{Deserialize, Serialize};

/// A physical card, distinct from its logical card and printing. Hidden cards
/// do not expose this identifier in player observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardId(pub(super) u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct PositionId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardKind {
    Bran,
    Palisade,
    SureGamble,
    HedgeFund,
    GovernmentSubsidy,
    OffworldOffice,
    SeamlessLaunch,
    Cleaver,
    Unity,
    DocklandsPass,
    #[cfg(test)]
    ConcealedFixture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Zone {
    Hq,
    Installing,
    ProtectingHq,
    Archives,
    Rd,
    Grip,
    Stack,
    Heap,
    Rig,
    CorpScore,
    RunnerScore,
    Root(Server),
    Protecting(Server),
    PlayArea,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CardInstance {
    pub id: CardId,
    pub kind: CardKind,
    pub zone: Zone,
    pub rezzed: bool,
    #[serde(default)]
    pub advancement: u32,
    #[serde(default)]
    pub installed_turn: Option<u64>,
    #[serde(default)]
    pub installed_by: Option<Player>,
    #[serde(default)]
    pub faceup: bool,
}

impl CardInstance {
    pub fn new(id: CardId, kind: CardKind, zone: Zone, rezzed: bool) -> Self {
        Self {
            id,
            kind,
            zone,
            rezzed,
            advancement: 0,
            installed_turn: None,
            installed_by: None,
            faceup: false,
        }
    }
    pub fn view(&self, viewer: Player) -> CardView {
        let visible = viewer == Player::Corp || self.rezzed || self.zone == Zone::Archives;
        let (title, printing, text) = match self.kind {
            CardKind::Bran => ("Brân 1.0", "30039", BRAN_TEXT),
            CardKind::Palisade => ("Palisade", "30072", PALISADE_TEXT),
            kind => kind.description(),
        };
        let mut view = CardView::face(title, printing, text, visible, self.rezzed);
        view.instance = visible.then_some(self.id);
        view
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct IcePosition {
    pub id: PositionId,
    pub card: Option<CardId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Run {
    pub position: PositionId,
    pub encountered: Option<CardId>,
}

/// Concrete continuation payloads for the admitted Brân interaction. The source
/// and installing card survive the nested Corp choice independently of position.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum Pending {
    BranInstall {
        source: CardId,
        card: CardId,
    },
    BranInstalling {
        source: CardId,
        card: CardId,
        position: PositionId,
    },
    Movement,
    Rez {
        ice: CardId,
    },
    Paid {
        priority: Player,
        passed: bool,
        window: TurnWindow,
    },
    Action {
        actor: Player,
    },
    Discard {
        actor: Player,
    },
    InstallDestination {
        actor: Player,
        card: CardId,
    },
    InstallTrash {
        actor: Player,
        card: CardId,
        server: Server,
    },
    SeamlessTarget,
    AccessOffice {
        card: CardId,
    },
}

impl Pending {
    pub fn actor(self) -> Player {
        match self {
            Self::Movement | Self::AccessOffice { .. } => Player::Runner,
            Self::Paid { priority, .. } => priority,
            Self::Action { actor }
            | Self::Discard { actor }
            | Self::InstallDestination { actor, .. }
            | Self::InstallTrash { actor, .. } => actor,
            _ => Player::Corp,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct State {
    pub corp_credits: u32,
    #[serde(default)]
    pub turn: Option<Turn>,
    pub revision: u64,
    pub cards: Vec<CardInstance>,
    /// Innermost first; a newly declared install position can still be vacant.
    pub hq_positions: Vec<IcePosition>,
    pub run: Option<Run>,
    pub pending: Option<Pending>,
    pub run_result: Option<RunResult>,
    pub history: Vec<[String; 2]>,
}

impl State {
    /// Check references before any runtime method may rely on them. This admits
    /// the currently implemented interaction; it is not a general save validator.
    pub fn valid(&self) -> bool {
        if self.turn.is_some() {
            return self.valid_turns();
        }
        // Admission still covers only the accepted two-card position. Merely
        // decoding another copy must not turn this into unsupported ordinary play.
        if self.cards.len() != 2
            || self
                .cards
                .iter()
                .filter(|card| card.kind == CardKind::Bran)
                .count()
                != 1
            || self
                .cards
                .iter()
                .filter(|card| card.kind == CardKind::Palisade)
                .count()
                != 1
        {
            return false;
        }
        for (index, card) in self.cards.iter().enumerate() {
            if self.cards[..index].iter().any(|other| other.id == card.id)
                || (card.rezzed && card.zone != Zone::ProtectingHq)
            {
                return false;
            }
            let positions = self
                .hq_positions
                .iter()
                .filter(|position| position.card == Some(card.id))
                .count();
            if positions != usize::from(card.zone == Zone::ProtectingHq) {
                return false;
            }
        }
        for (index, position) in self.hq_positions.iter().enumerate() {
            if self.hq_positions[..index]
                .iter()
                .any(|other| other.id == position.id)
                || position
                    .card
                    .is_some_and(|id| !self.cards.iter().any(|card| card.id == id))
            {
                return false;
            }
        }
        let vacant = self
            .hq_positions
            .iter()
            .filter(|position| position.card.is_none())
            .count();
        let installing = self
            .cards
            .iter()
            .filter(|card| card.zone == Zone::Installing)
            .count();
        if self.run_result.is_some() {
            return self.run.is_none() && self.pending.is_none() && vacant == 0 && installing == 0;
        }
        let (Some(run), Some(pending)) = (&self.run, self.pending) else {
            return false;
        };
        let Some(current) = self
            .hq_positions
            .iter()
            .position(|position| position.id == run.position)
        else {
            return false;
        };
        let card_at_run = self.hq_positions[current].card;
        let is_card = |id, kind, zone| {
            self.cards
                .iter()
                .any(|card| card.id == id && card.kind == kind && card.zone == zone)
        };
        let bran_trashed = self
            .cards
            .iter()
            .any(|card| card.kind == CardKind::Bran && card.zone == Zone::Archives);
        // Admission promises that every remaining choice in this bounded run
        // can advance the revision and yield another resumable checkpoint.
        let remaining_choices = match pending {
            Pending::BranInstall { .. } => 5,
            Pending::BranInstalling { .. } => 4,
            Pending::Movement if current > 0 => 3,
            Pending::Rez { .. } => 2,
            Pending::Movement => 1,
            _ => return false,
        };
        if self.revision.checked_add(remaining_choices).is_none() {
            return false;
        }
        match pending {
            Pending::BranInstall { source, card } => {
                is_card(source, CardKind::Bran, Zone::ProtectingHq)
                    && is_card(card, CardKind::Palisade, Zone::Hq)
                    && card_at_run == Some(source)
                    && run.encountered == Some(source)
                    && self.card(source).rezzed
                    && vacant == 0
                    && installing == 0
                    && self.next_position_id().is_some()
            }
            Pending::BranInstalling {
                source,
                card,
                position,
            } => {
                is_card(source, CardKind::Bran, Zone::ProtectingHq)
                    && is_card(card, CardKind::Palisade, Zone::Installing)
                    && card_at_run == Some(source)
                    && run.encountered == Some(source)
                    && self.card(source).rezzed
                    && vacant == 1
                    && installing == 1
                    && current.checked_sub(1).is_some_and(|inward| {
                        self.hq_positions[inward].id == position
                            && self.hq_positions[inward].card.is_none()
                    })
            }
            Pending::Movement => {
                bran_trashed
                    && self.cards.iter().all(|card| !card.rezzed)
                    && run.encountered.is_none()
                    && installing == 0
                    && vacant == usize::from(card_at_run.is_none())
                    && card_at_run.map_or(current > 0, |id| !self.card(id).rezzed)
                    && current
                        .checked_sub(1)
                        .is_none_or(|inward| self.hq_positions[inward].card.is_some())
            }
            Pending::Rez { ice } => {
                bran_trashed
                    && is_card(ice, CardKind::Palisade, Zone::ProtectingHq)
                    && card_at_run == Some(ice)
                    && run.encountered.is_none()
                    && !self.card(ice).rezzed
                    && vacant == 0
                    && installing == 0
            }
            _ => false,
        }
    }

    pub fn next_position_id(&self) -> Option<PositionId> {
        self.hq_positions
            .iter()
            .map(|position| position.id.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .map(PositionId)
    }

    pub fn card(&self, id: CardId) -> &CardInstance {
        self.cards
            .iter()
            .find(|card| card.id == id)
            .expect("admitted card")
    }

    pub fn card_mut(&mut self, id: CardId) -> &mut CardInstance {
        self.cards
            .iter_mut()
            .find(|card| card.id == id)
            .expect("admitted card")
    }

    pub fn current_position(&self) -> usize {
        let id = self.run.as_ref().expect("active run").position;
        self.hq_positions
            .iter()
            .position(|position| position.id == id)
            .expect("admitted run position")
    }

    pub fn finish_run(&mut self, result: RunResult) {
        self.run_result = Some(result);
        self.pending = None;
        self.run = None;
        self.hq_positions.retain(|position| position.card.is_some());
    }
}

/// Authoritative continuation for the controller, never an input to a player or
/// bot. This is a game payload, not a filesystem save or a compatibility promise.
#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub(super) version: u32,
    pub(super) rules: String,
    pub(super) card_revision: String,
    pub(super) state: State,
}

pub(super) const RULES: &str = "26.03";
pub(super) const CARD_REVISION: &str = "6ff880724741ea6ac062acf77e0cf979bb44e67e";
