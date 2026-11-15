//! Netrunner-owned card locations and suspended run decisions. Position identity
//! follows CR 26.03 §§6.2.4–6.2.6, including a vacant position occupied by Runner.
use super::turns::{Server, Turn, TurnWindow};
use super::{Player, RunResult};
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
    Whitespace,
    Overclock,
    SureGamble,
    HedgeFund,
    GovernmentSubsidy,
    OffworldOffice,
    SeamlessLaunch,
    Cleaver,
    Unity,
    DocklandsPass,
    ManegarmSkunkworks,
    // Remaining introductory card kinds; no booster cards are admitted.
    TreadLightly,
    CreativeCommission,
    VRcation,
    Jailbreak,
    Pennyshaver,
    Carmen,
    Mayfly,
    RedTeam,
    TeleworkContract,
    SmartwareDistributor,
    VerbalPlasticity,
    SendAMessage,
    SuperconductingHub,
    NicoCampaign,
    UrticaCipher,
    RegolithMiningLicense,
    Diviner,
    Karuna,
    Tithe,

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
    #[serde(default)]
    pub hosted_credits: u32,
    #[serde(default)]
    pub telework_used_turn: Option<u64>,
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
            hosted_credits: 0,
            telework_used_turn: None,
        }
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

/// Concrete ordinary turn continuations. Run-specific suspension lives in
/// turns::runs and enters here through RunStep.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum Pending {
    Mulligan {
        actor: Player,
    },
    Paid {
        priority: Player,
        passed: bool,
        window: TurnWindow,
    },
    Action {
        actor: Player,
    },
    TurnIncome {
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
    HubDraw {
        card: CardId,
        window: TurnWindow,
    },
    SendMessage {
        card: CardId,
        window: TurnWindow,
    },
    RunServer,
    RunStep {
        actor: Player,
    },
    AccessOffice {
        card: CardId,
    },
}

impl Pending {
    pub fn actor(self) -> Player {
        match self {
            Self::RunServer | Self::AccessOffice { .. } => Player::Runner,
            Self::Paid { priority, .. } => priority,
            Self::RunStep { actor } => actor,
            Self::Action { actor }
            | Self::Mulligan { actor }
            | Self::TurnIncome { actor }
            | Self::Discard { actor }
            | Self::InstallDestination { actor, .. }
            | Self::InstallTrash { actor, .. } => actor,
            _ => Player::Corp,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct State {
    /// Changes on a draw/access even when an already known card is encountered.
    #[serde(default)]
    pub information_revision: u64,
    pub corp_credits: u32,
    #[serde(default)]
    pub turn: Option<Turn>,
    pub revision: u64,
    pub cards: Vec<CardInstance>,
    /// Retained empty fields in checkpoint v2. Legacy run records are refused;
    /// all live run positions belong to turn.run.
    pub hq_positions: Vec<IcePosition>,
    pub run: Option<Run>,
    pub pending: Option<Pending>,
    pub run_result: Option<RunResult>,
    pub history: Vec<[String; 2]>,
}

impl State {
    /// Check the ordinary match references before runtime methods rely on them.
    /// Legacy turn=None payloads are unsupported, never reinterpreted.
    pub fn valid(&self) -> bool {
        if self.information_revision > self.revision {
            return false;
        }
        self.turn.is_some() && self.valid_turns()
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
}

/// Authoritative continuation for the controller, never an input to a player or
/// bot. This is a game payload, not a filesystem save or a compatibility promise.
#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub(super) version: u32,
    pub(super) rules: String,
    pub(super) card_revision: String,
    pub(super) state: State,
    #[serde(default)]
    pub(super) undo: super::undo::Correction,
}

pub(super) const RULES: &str = "26.03";
pub(super) const CARD_REVISION: &str = "6ff880724741ea6ac062acf77e0cf979bb44e67e";
