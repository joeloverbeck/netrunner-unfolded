//! System Gateway introductory matches and their engineering positions, using
//! CR 26.03. Every caller uses the same observations and validated transitions.

mod mechanics;
mod state;
pub use mechanics::{BreakerMechanics, IceEffect, IceFamily, IceMechanics};
mod turns;
mod undo;
use serde::{Deserialize, Serialize};
pub use state::{CardId, CardKind, Checkpoint, Zone};
use state::{CardInstance, Pending, State};
pub use turns::{
    EngineeringCard, EngineeringPosition, RunView, Server, ServerView, SubroutineView, TurnWindow,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Player {
    Corp,
    Runner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    KeepHand,
    Mulligan,
    DeclineInstall,
    Continue,
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
    StealAgenda,
    RunServer(Server),
    RunRedTeam { card: CardId, server: Server },
    TakePennyshaver(CardId),
    TakeTelework(CardId),
    TakeRegolith(CardId),
    LoadSmartware(CardId),
    ResolveIncome(CardId),
    Boost(CardId),
    Break { breaker: CardId, subroutines: u8 },
    BreakWithClick(u8),
    PayRunCredits(u8),
    InstallInward(CardId),
    PayManegarmCredits,
    SpendManegarmClicks,
    RefuseManegarm,
    AccessRoot(usize),
    AccessHq,
    AccessRd,
    RezIceFree(CardId),
    DeclineAgendaAbility,
    DrawAgendaCards,
    AccessArchives(usize),
    TrashAccessed,
    FinishAccess,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::KeepHand => "Keep opening hand",
            Self::Mulligan => "Redraw all five · once only",
            Self::DeclineInstall => "Decline installation",
            Self::Continue => "Continue run",
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
            Self::RunServer(_) => "Run server",
            Self::RunRedTeam { .. } => "Red Team · run an unvisited central · 1 click",
            Self::TakePennyshaver(_) => "Pennyshaver · place 1 credit, then take all · 1 click",
            Self::TakeTelework(_) => "Telework Contract · take 3 credits · 1 click, once per turn",
            Self::TakeRegolith(_) => "Regolith Mining License · take 3 credits · 1 click",
            Self::LoadSmartware(_) => "Smartware Distributor · place 3 credits · 1 click",
            Self::ResolveIncome(_) => "Resolve this turn-start income next",
            Self::Boost(_) => "Boost icebreaker strength",
            Self::Break { .. } => "Break selected subroutines",
            Self::BreakWithClick(_) => "Lose 1 click to break Brân subroutine",
            Self::PayRunCredits(_) => "Pay run credits",
            Self::InstallInward(_) => "Install inward for free",
            Self::PayManegarmCredits => "Manegarm · pay 5 credits; choose sources",
            Self::SpendManegarmClicks => "Manegarm · spend 2 clicks; continue run",
            Self::RefuseManegarm => "Decline Manegarm payment · end the run",
            Self::AccessRoot(_) => "Access root card",
            Self::AccessHq => "Access a random HQ card",
            Self::RezIceFree(_) => "Rez installed ICE · free",
            Self::DrawAgendaCards => "Draw 2 cards · Superconducting Hub",
            Self::DeclineAgendaAbility => "Decline agenda ability",
            Self::AccessRd => "Access the next R&D card",
            Self::AccessArchives(_) => "Access Archives card",
            Self::TrashAccessed => "Trash accessed card · pay 3 credits",
            Self::FinishAccess => "Leave card · finish access",
            Self::StealAgenda => "Steal accessed agenda",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub actor: Player,
    pub revision: u64,
    pub choices: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunResult {
    EndedByIce,
    EndedByManegarm,
    SuccessfulEmptyHq,
    JackedOut,
    SuccessfulAccess,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardView {
    pub instance: Option<CardId>,
    pub title: Option<String>,
    pub rezzed: bool,
    #[serde(deserialize_with = "observation_rule_text")]
    pub printing: Option<&'static str>,
    #[serde(deserialize_with = "observation_rule_text")]
    pub rules_text: Option<&'static str>,
    pub advancement: u32,
    pub hosted_credits: u32,
    pub selected: bool,
    /// Public physical orientation, independent of an owner's right to inspect.
    pub face_up: bool,
    /// Public installation timing, needed for effects such as Seamless Launch.
    #[serde(default)]
    pub installed_this_turn: bool,
}

// Only the short-lived, size-bounded observation diagnostic deserializes these
// views. JSON escapes (notably multiline ICE text) cannot borrow their bytes.
// Retain this text for that process lifetime; authoritative saves use State.
fn observation_rule_text<'de, D>(deserializer: D) -> Result<Option<&'static str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
        .map(|text| text.map(|text| &*Box::leak(text.into_boxed_str())))
}

fn observation_text<'de, D>(deserializer: D) -> Result<&'static str, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer).map(|text| &*Box::leak(text.into_boxed_str()))
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
            hosted_credits: 0,
            selected: false,
            face_up: visible,
            installed_this_turn: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub run: Option<RunView>,
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
    pub play_area: Vec<CardView>,
    pub corp_score: Vec<CardView>,
    pub runner_score: Vec<CardView>,
    pub servers: Vec<ServerView>,
    pub memory_used: u8,
    pub memory_capacity: u8,
    pub central_runs: Vec<Server>,
    pub runner_basic_draw_taken: bool,
    #[serde(default)]
    pub successful_run_this_turn: bool,
    #[serde(default)]
    pub hq_breached_this_turn: bool,
    #[serde(deserialize_with = "observation_rule_text")]
    pub terminal_reason: Option<&'static str>,
    pub accessed: Option<CardView>,
    pub install_destination: Option<Server>,
    pub install_cost: Option<u32>,
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
    #[serde(deserialize_with = "observation_text")]
    pub phase: &'static str,
}

pub struct Match(State, undo::Correction);

impl Match {
    pub(crate) fn revision(&self) -> u64 {
        self.0.revision
    }
    pub(crate) fn has_turns(&self) -> bool {
        self.0.turn.is_some()
    }
    /// Capture a suspended decision without resolving it or replaying actions.
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            version: 2,
            rules: state::RULES.into(),
            card_revision: state::CARD_REVISION.into(),
            state: self.0.clone(),
            undo: self.1.clone(),
        }
    }

    /// Restore only a supported, coherent game payload. Player observations
    /// remain the only interface to pass to a human seat or opponent policy.
    pub fn resume(checkpoint: Checkpoint) -> Result<Self, &'static str> {
        if checkpoint.version != 2
            || checkpoint.state.turn.is_none()
            || checkpoint.rules != state::RULES
            || checkpoint.card_revision != state::CARD_REVISION
        {
            return Err("Unsupported match continuation.");
        }
        if !checkpoint.state.valid() || !checkpoint.undo.valid(&checkpoint.state) {
            return Err("Invalid match continuation.");
        }
        Ok(Self(checkpoint.state, checkpoint.undo))
    }

    /// Regression input only: set up Brân and enter its subroutine through the
    /// ordinary turn/run transitions. This is not a separate rules mode.
    pub fn prepared_bran_run(corp_credits: u8) -> Self {
        let mut position = EngineeringPosition::economy(1);
        position.active = Player::Runner;
        position.corp_credits = corp_credits.into();
        position.runner_credits = 0;
        position.cards = vec![
            EngineeringCard {
                kind: CardKind::Bran,
                zone: Zone::Protecting(Server::Hq),
                rezzed: true,
                advancement: 0,
                installed_turn: Some(1),
            },
            EngineeringCard::new(CardKind::Palisade, Zone::Hq),
        ];
        let mut game = Self::engineering_turns(position).expect("Brân regression input");
        // This independently specified starting position has one click left;
        // initiating the run spends it, leaving no click-break opportunity.
        game.0.turn.as_mut().unwrap().clicks[1] = 1;
        for _ in 0..20 {
            let actor = [Player::Corp, Player::Runner]
                .into_iter()
                .find(|&p| game.observe(p).decision.is_some())
                .unwrap();
            let decision = game.observe(actor).decision.unwrap();
            if decision.choices.contains(&Action::DeclineInstall) {
                game.1 = undo::Correction::default();
                return game;
            }
            let action = if decision.choices.contains(&Action::RunServer(Server::Hq)) {
                Action::RunServer(Server::Hq)
            } else {
                Action::PassPriority
            };
            game.choose(actor, decision.revision, action)
                .expect("Brân setup transition");
        }
        unreachable!("Brân subroutine reached within the bounded setup")
    }
    pub fn observe(&self, viewer: Player) -> Observation {
        self.observe_turns(viewer)
    }
    pub fn choose(
        &mut self,
        actor: Player,
        revision: u64,
        action: Action,
    ) -> Result<(), &'static str> {
        let forced = self
            .observe(actor)
            .decision
            .is_some_and(|d| d.choices == [Action::PassPriority]);
        let before = self.0.clone();
        self.choose_inner(actor, revision, action)?;
        self.record_correction(before, actor, forced);
        Ok(())
    }
    fn choose_inner(
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
        self.choose_turns(actor, action)
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
    let policy = match observation.decision.as_ref()?.actor {
        Player::Corp => crate::policies::Policy::CorpV0,
        Player::Runner => crate::policies::Policy::RunnerV0,
    };
    policy.choose(observation)
}
