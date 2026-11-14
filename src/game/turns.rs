//! Ordinary turn timing in the same match. CR 26.03 chapters 5, 8.5, 9.2 and
//! 10.3; card text from the pinned official snapshot. Engineering positions do
//! not admit complete introductory decks or expose an additional playable mode.
use super::*;
use unfolded_tabletop::random::SeededRandom;
mod engineering;
mod runs;
pub use runs::{RunView, SubroutineView};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Server {
    #[default]
    Hq,
    Rd,
    Archives,
    Remote(u32),
    NewRemote,
    Rig,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ServerView {
    pub server: Server,
    pub root: Vec<CardView>,
    pub ice: Vec<CardView>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnWindow {
    Start,
    BeforeAction,
    End,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Turn {
    pub active: Player,
    pub number: u64,
    pub clicks: [u8; 2],
    pub runner_credits: u32,
    pub points: [u8; 2],
    pub winner: Option<Player>,
    pub terminal: Option<Terminal>,
    pub selected: Vec<CardId>,
    pub random_state: u64,
    pub random_algorithm: String,
    pub next_remote: u32,
    #[serde(default)]
    pub run: Option<runs::ActiveRun>,
    pub hq_breached: bool,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) enum Terminal {
    AgendaPoints,
    CorpDrawFailure,
}
impl Player {
    pub fn other(self) -> Self {
        match self {
            Self::Corp => Self::Runner,
            Self::Runner => Self::Corp,
        }
    }
    pub fn index(self) -> usize {
        usize::from(self == Self::Runner)
    }
}
impl Server {
    pub fn label(self) -> String {
        match self {
            Self::Hq => "HQ".into(),
            Self::Rd => "R&D".into(),
            Self::Archives => "Archives".into(),
            Self::Remote(id) => format!("Remote {id}"),
            Self::NewRemote => "New remote server".into(),
            Self::Rig => "Runner rig".into(),
        }
    }
}
impl Action {
    pub fn description(self, view: &Observation) -> String {
        if self == Self::PassPriority
            && let Some(run) = &view.run
        {
            return if run.encounter_paid {
                if run.subroutines.iter().all(|s| s.broken) {
                    "Finish encounter".into()
                } else {
                    "Resolve unbroken".into()
                }
            } else {
                "No more abilities · pass".into()
            };
        }
        if self == Self::PassPriority {
            return "No more abilities · pass".into();
        }
        match self {
            Self::AccessRoot(index) => {
                return format!(
                    "Access {} root card {}",
                    view.run
                        .as_ref()
                        .map(|r| r.server.label())
                        .unwrap_or_default(),
                    index + 1
                );
            }
            Self::RunServer(server) => {
                return format!(
                    "Run {}{}",
                    server.label(),
                    if view.phase == "Action phase" {
                        " · 1 click"
                    } else {
                        ""
                    }
                );
            }
            Self::PayRunCredits(hosted) => {
                return format!(
                    "Pay {} pool + {hosted} Overclock",
                    view.run.as_ref().and_then(|r| r.payment_cost).unwrap_or(0) - hosted
                );
            }
            Self::BreakWithClick(n) => return format!("Brân · lose 1 click: break subroutine {n}"),
            Self::Boost(id) | Self::Break { breaker: id, .. } => {
                let title = view
                    .rig
                    .iter()
                    .find(|c| c.instance == Some(id))
                    .and_then(|c| c.title.as_deref())
                    .unwrap_or("Icebreaker");
                return match self {
                    Self::Boost(_) => format!(
                        "{title} · +{} strength · {} credit{}{}",
                        if title == "Cleaver" {
                            1
                        } else {
                            view.rig
                                .iter()
                                .filter(|c| matches!(c.printing, Some("30006" | "30026")))
                                .count()
                        },
                        if title == "Cleaver" { 2 } else { 1 },
                        if title == "Cleaver" { "s" } else { "" },
                        if view.run.as_ref().is_some_and(|r| r.encounter_paid) {
                            ""
                        } else {
                            " · expires immediately"
                        }
                    ),
                    Self::Break { subroutines, .. } => format!(
                        "{title} · break {} · 1 credit",
                        (0..3)
                            .filter(|n| subroutines & (1 << n) != 0)
                            .map(|n| format!("sub {}", n + 1))
                            .collect::<Vec<_>>()
                            .join(" + ")
                    ),
                    _ => unreachable!(),
                };
            }
            _ => {}
        }
        if self == Self::FinishInstall && view.run.is_some() {
            return "Finish inward installation · free".into();
        }
        if self == Self::FinishInstall {
            return format!(
                "Finish installing {} at {} · {} credits",
                view.installing
                    .as_ref()
                    .and_then(|card| card.title.as_deref())
                    .unwrap_or("card"),
                view.install_destination
                    .map(Server::label)
                    .unwrap_or_default(),
                view.install_cost.unwrap_or(0)
            );
        }
        if let Self::Destination(server) = self {
            return format!("Install at {}", server.label());
        }
        let id = match self {
            Self::PlayCard(id)
            | Self::InstallCard(id)
            | Self::TrashInstalled(id)
            | Self::Advance(id)
            | Self::Score(id)
            | Self::Rez(id)
            | Self::SelectTarget(id)
            | Self::ToggleDiscard(id)
            | Self::InstallInward(id) => id,
            _ => return self.label().into(),
        };
        let card = view
            .hand
            .iter()
            .chain(&view.rig)
            .chain(&view.ice)
            .chain(&view.archives)
            .chain(view.installing.iter())
            .chain(
                view.servers
                    .iter()
                    .flat_map(|s| s.root.iter().chain(&s.ice)),
            )
            .find(|card| card.instance == Some(id));
        let title = card.and_then(|c| c.title.as_deref()).unwrap_or("card");
        let location = view
            .servers
            .iter()
            .find_map(|server| {
                server
                    .root
                    .iter()
                    .position(|c| c.instance == Some(id))
                    .map(|index| format!(" · {} root {}", server.server.label(), index + 1))
                    .or_else(|| {
                        server
                            .ice
                            .iter()
                            .position(|c| c.instance == Some(id))
                            .map(|index| format!(" · {} ICE {}", server.server.label(), index + 1))
                    })
            })
            .or_else(|| {
                view.rig
                    .iter()
                    .position(|c| c.instance == Some(id))
                    .map(|index| format!(" · rig {}", index + 1))
            })
            .unwrap_or_default();
        match self {
            Self::PlayCard(_) => format!(
                "Play {title} · 1 click + {} credits",
                match card.and_then(|c| c.printing) {
                    Some("30064") => 10,
                    Some("30040" | "30029") => 1,
                    _ => 5,
                }
            ),
            Self::InstallInward(_) => format!("Install {title} directly inward · free"),
            Self::Rez(_) => format!(
                "Rez {title} · {} credits",
                match card.and_then(|c| c.printing) {
                    Some("30039") => 6,
                    Some("30072") => 3,
                    _ => 2,
                }
            ),
            Self::InstallCard(_) => {
                format!("Install {title} · 1 click; choose destination and costs")
            }
            Self::TrashInstalled(_) => format!("Trash {title}{location}"),
            Self::Advance(_) => format!("Advance {title}{location} · 1 click + 1 credit"),
            Self::Score(_) => format!("Score {title}{location} · 2 points"),
            Self::SelectTarget(_) => format!("Place 2 counters on {title}{location}"),
            Self::ToggleDiscard(_) => format!(
                "{} {title}",
                if card.is_some_and(|c| c.selected) {
                    "Keep"
                } else {
                    "Discard"
                }
            ),
            _ => self.label().into(),
        }
    }
}
pub(super) fn baseline(view: &Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if let Some(run) = &view.run {
        for action in [
            Action::StealAgenda,
            Action::SpendManegarmClicks,
            Action::PayManegarmCredits,
            Action::TrashAccessed,
            Action::AccessRoot(0),
            Action::AccessHq,
            Action::FinishAccess,
        ] {
            if choices.contains(&action) {
                return Some(action);
            }
        }
        if let Some(action) = choices
            .iter()
            .filter(|a| matches!(a, Action::PayRunCredits(_)))
            .max_by_key(|a| {
                if let Action::PayRunCredits(n) = a {
                    *n
                } else {
                    0
                }
            })
        {
            return Some(*action);
        }
        if let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(_))) {
            return Some(*action);
        }
        if choices.contains(&Action::DeclineInstall) {
            return Some(Action::DeclineInstall);
        }
        if choices.contains(&Action::FinishInstall) {
            return Some(Action::FinishInstall);
        }
        if let Some(action) = choices
            .iter()
            .filter(|a| matches!(a, Action::Break { .. }))
            .max_by_key(|a| {
                if let Action::Break { subroutines, .. } = a {
                    subroutines.count_ones()
                } else {
                    0
                }
            })
        {
            return Some(*action);
        }
        if let Some(action) = choices
            .iter()
            .find(|a| matches!(a, Action::BreakWithClick(2 | 3)))
        {
            return Some(*action);
        }
        if run.encounter_paid
            && let Some(ice) = &run.ice
            && let Some((id, _, _)) = run.breakers.iter().find(|(_, title, strength)| {
                *strength < run.ice_strength.unwrap_or(0)
                    && ((title == "Cleaver" && matches!(ice.printing, Some("30039" | "30072")))
                        || (title == "Unity" && ice.printing == Some("30074")))
            })
            && choices.contains(&Action::Boost(*id))
        {
            return Some(Action::Boost(*id));
        }
        if choices.contains(&Action::Continue) {
            return Some(Action::Continue);
        }
        if choices.contains(&Action::PassPriority) {
            return Some(Action::PassPriority);
        }
        return choices.first().copied();
    }
    let card = |id| {
        view.hand
            .iter()
            .chain(&view.rig)
            .chain(
                view.servers
                    .iter()
                    .flat_map(|s| s.root.iter().chain(&s.ice)),
            )
            .find(|c| c.instance == Some(id))
    };
    // Resolve pending obligations before planning another ordinary action.
    for action in choices {
        if matches!(
            action,
            Action::ConfirmDiscard | Action::FinishInstall | Action::StealAgenda | Action::Score(_)
        ) {
            return Some(*action);
        }
    }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::ToggleDiscard(id) if card(*id).is_some_and(|c| !c.selected)))
    {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::SelectTarget(id) if card(*id).is_some_and(|c| c.printing == Some("30067") && c.advancement < 4))) { return Some(*action); }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::SelectTarget(_) | Action::TrashInstalled(_)))
    {
        return Some(*action);
    }
    if choices.contains(&Action::Destination(Server::NewRemote)) {
        return Some(Action::Destination(Server::NewRemote));
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::Destination(_))) {
        return Some(*action);
    }
    // Prefer two counters for one credit when a visible agenda needs them.
    if view.servers.iter().flat_map(|s| &s.root).any(|c| c.printing == Some("30067") && (2..4).contains(&c.advancement))
        && let Some(action) = choices.iter().find(|a| matches!(a,Action::PlayCard(id) if card(*id).is_some_and(|c| c.printing == Some("30040")))) {
        return Some(*action);
    }
    // Economic operations precede installing and advancing visible agendas.
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::PlayCard(id) if card(*id).is_some_and(|c| matches!(c.printing, Some("30030" | "30075" | "30064"))))) { return Some(*action); }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::Advance(id) if card(*id).is_some_and(|c| c.advancement < 4)))
    {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::InstallCard(id) if card(*id).is_some_and(|c| matches!(c.printing, Some("30067" | "30006" | "30026" | "30013"))))) { return Some(*action); }
    if choices.contains(&Action::GainCredit) {
        return Some(Action::GainCredit);
    }
    if choices.contains(&Action::PassPriority) {
        return Some(Action::PassPriority);
    }
    choices.first().copied()
}
impl CardKind {
    pub fn description(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Bran => ("Brân 1.0", "30039", BRAN_TEXT),
            Self::Palisade => ("Palisade", "30072", PALISADE_TEXT),
            Self::Whitespace => (
                "Whitespace",
                "30074",
                "Subroutine 1: The Runner loses 3 credits.\nSubroutine 2: If the Runner has 6 credits or less, end the run.\nRez: 2 credits. Strength: 0. Code gate.",
            ),
            Self::Overclock => (
                "Overclock",
                "30029",
                "Place 5 credits on this event, then run any server. You can spend hosted credits during that run. Play cost: 1 credit.",
            ),
            Self::SureGamble => (
                "Sure Gamble",
                "30030",
                "Gain 9 credits. Play cost: 5 credits.",
            ),
            Self::HedgeFund => (
                "Hedge Fund",
                "30075",
                "Gain 9 credits. Play cost: 5 credits.",
            ),
            Self::GovernmentSubsidy => (
                "Government Subsidy",
                "30064",
                "Gain 15 credits. Play cost: 10 credits.",
            ),
            Self::OffworldOffice => (
                "Offworld Office",
                "30067",
                "When you score this agenda, gain 7 credits. Advancement requirement: 4. Agenda points: 2.",
            ),
            Self::SeamlessLaunch => (
                "Seamless Launch",
                "30040",
                "Place 2 advancement counters on 1 installed card that you did not install this turn. Play cost: 1 credit.",
            ),
            Self::Cleaver => (
                "Cleaver",
                "30006",
                "Interface → 1 credit: Break up to 2 barrier subroutines. 2 credits: +1 strength. Install: 3 credits. Memory: 1.",
            ),
            Self::Unity => (
                "Unity",
                "30026",
                "Interface → 1 credit: Break 1 code gate subroutine. 1 credit: +X strength. X is the number of installed icebreakers (including this one). Install: 3 credits. Memory: 1.",
            ),
            Self::TreadLightly => (
                "Tread Lightly",
                "30012",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::CreativeCommission => (
                "Creative Commission",
                "30020",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::VRcation => (
                "VRcation",
                "30021",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Jailbreak => (
                "Jailbreak",
                "30028",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Pennyshaver => (
                "Pennyshaver",
                "30014",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Carmen => (
                "Carmen",
                "30015",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Mayfly => (
                "Mayfly",
                "30032",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::RedTeam => (
                "Red Team",
                "30018",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::TeleworkContract => (
                "Telework Contract",
                "30027",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::SmartwareDistributor => (
                "Smartware Distributor",
                "30033",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::VerbalPlasticity => (
                "Verbal Plasticity",
                "30034",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::SendAMessage => (
                "Send a Message",
                "30069",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::SuperconductingHub => (
                "Superconducting Hub",
                "30070",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::NicoCampaign => (
                "Nico Campaign",
                "30037",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::UrticaCipher => (
                "Urtica Cipher",
                "30045",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::RegolithMiningLicense => (
                "Regolith Mining License",
                "30071",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Diviner => (
                "Diviner",
                "30046",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Karuna => (
                "Karunā",
                "30047",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::Tithe => (
                "Tithe",
                "30073",
                "Unsupported card: inactive engineering deck remainder only.",
            ),
            Self::ManegarmSkunkworks => (
                "Manegarm Skunkworks",
                "30042",
                "Whenever the Runner approaches this server, end the run unless they spend 2 clicks or pay 5 credits. Rez: 2 credits. Trash: 3 credits. Region.",
            ),
            Self::DocklandsPass => (
                "Docklands Pass",
                "30013",
                "The first time each turn you breach HQ, access 1 additional card. Unique. Install: 2 credits.",
            ),
            #[cfg(test)]
            Self::ConcealedFixture => ("Unrevealed fixture", "fixture", "Private fixture text"),
        }
    }
    fn unsupported(self) -> bool {
        matches!(
            self,
            Self::TreadLightly
                | Self::CreativeCommission
                | Self::VRcation
                | Self::Jailbreak
                | Self::Pennyshaver
                | Self::Carmen
                | Self::Mayfly
                | Self::RedTeam
                | Self::TeleworkContract
                | Self::SmartwareDistributor
                | Self::VerbalPlasticity
                | Self::SendAMessage
                | Self::SuperconductingHub
                | Self::NicoCampaign
                | Self::UrticaCipher
                | Self::RegolithMiningLicense
                | Self::Diviner
                | Self::Karuna
                | Self::Tithe
        )
    }
    fn owner(self) -> Player {
        if matches!(
            self,
            Self::Overclock
                | Self::SureGamble
                | Self::Cleaver
                | Self::Unity
                | Self::DocklandsPass
                | Self::TreadLightly
                | Self::CreativeCommission
                | Self::VRcation
                | Self::Jailbreak
                | Self::Pennyshaver
                | Self::Carmen
                | Self::Mayfly
                | Self::RedTeam
                | Self::TeleworkContract
                | Self::SmartwareDistributor
                | Self::VerbalPlasticity
        ) {
            Player::Runner
        } else {
            Player::Corp
        }
    }
    fn memory(self) -> u8 {
        match self {
            Self::Cleaver | Self::Unity => 1,
            _ => 0,
        }
    }
    fn ice(self) -> bool {
        matches!(self, Self::Bran | Self::Palisade | Self::Whitespace)
    }
    fn installable(self) -> bool {
        self.ice()
            || matches!(
                self,
                Self::OffworldOffice
                    | Self::Cleaver
                    | Self::Unity
                    | Self::DocklandsPass
                    | Self::ManegarmSkunkworks
            )
    }
    fn play_cost(self) -> Option<u32> {
        match self {
            Self::SureGamble | Self::HedgeFund => Some(5),
            Self::GovernmentSubsidy => Some(10),
            Self::SeamlessLaunch | Self::Overclock => Some(1),
            _ => None,
        }
    }
    fn install_cost(self) -> u32 {
        match self {
            Self::Cleaver | Self::Unity => 3,
            Self::DocklandsPass => 2,
            _ => 0,
        }
    }
}

/// Explicit source-derived engineering input. No ordinary UI can create one;
/// production loading and every subsequent choice use the normal match path.
#[derive(Clone)]
pub struct EngineeringCard {
    pub kind: CardKind,
    pub zone: Zone,
    pub advancement: u32,
    pub installed_turn: Option<u64>,
    pub rezzed: bool,
}
impl EngineeringCard {
    pub fn new(kind: CardKind, zone: Zone) -> Self {
        Self {
            kind,
            zone,
            advancement: 0,
            installed_turn: None,
            rezzed: false,
        }
    }
}
#[derive(Clone)]
pub struct EngineeringPosition {
    pub active: Player,
    pub corp_credits: u32,
    pub runner_credits: u32,
    pub agenda_points: [u8; 2],
    pub cards: Vec<EngineeringCard>,
    pub seed: u64,
    pub shuffle_decks: bool,
    /// Start at the actual mandatory steal decision, after access has revealed
    /// a remote Office. This does not implement a run initiation shortcut.
    pub accessed_office: bool,
}
impl EngineeringPosition {
    pub fn economy(seed: u64) -> Self {
        use CardKind::*;
        Self {
            active: Player::Corp,
            corp_credits: 5,
            runner_credits: 5,
            agenda_points: [0, 0],
            seed,
            shuffle_decks: true,
            accessed_office: false,
            cards: vec![
                EngineeringCard::new(HedgeFund, Zone::Hq),
                EngineeringCard::new(OffworldOffice, Zone::Hq),
                EngineeringCard::new(SeamlessLaunch, Zone::Rd),
                EngineeringCard::new(GovernmentSubsidy, Zone::Rd),
                EngineeringCard::new(HedgeFund, Zone::Rd),
                EngineeringCard::new(SureGamble, Zone::Grip),
                EngineeringCard::new(Cleaver, Zone::Grip),
                EngineeringCard::new(Unity, Zone::Grip),
                EngineeringCard::new(DocklandsPass, Zone::Grip),
                EngineeringCard::new(SureGamble, Zone::Stack),
            ],
        }
    }
}
impl Match {
    pub fn engineering_turns(position: EngineeringPosition) -> Result<Self, &'static str> {
        let mut cards: Vec<_> = position
            .cards
            .into_iter()
            .enumerate()
            .map(|(i, input)| {
                let mut card =
                    CardInstance::new(CardId(i as u32 + 1), input.kind, input.zone, input.rezzed);
                card.advancement = input.advancement;
                card.installed_turn = input.installed_turn;
                card.installed_by = input.installed_turn.map(|_| input.kind.owner());
                card.faceup = input.rezzed
                    || matches!(
                        input.zone,
                        Zone::Heap | Zone::CorpScore | Zone::RunnerScore | Zone::Rig
                    );
                card
            })
            .collect();
        let mut random = SeededRandom::new(position.seed);
        if position.shuffle_decks {
            for zone in [Zone::Rd, Zone::Stack] {
                let indices: Vec<_> = cards
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| (c.zone == zone).then_some(i))
                    .collect();
                let mut shuffled: Vec<_> = indices.iter().map(|&i| cards[i].clone()).collect();
                random.shuffle(&mut shuffled);
                for (index, card) in indices.into_iter().zip(shuffled) {
                    cards[index] = card;
                }
            }
        }
        let next_remote = cards
            .iter()
            .filter_map(|c| match c.zone {
                Zone::Root(Server::Remote(id)) | Zone::Protecting(Server::Remote(id)) => Some(id),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Remote identifiers exhausted")?;
        let mut clicks = [0, 0];
        clicks[position.active.index()] = if position.active == Player::Corp {
            3
        } else {
            4
        };
        let pending = if position.accessed_office {
            Pending::AccessOffice {
                card: cards
                    .iter()
                    .find(|c| c.kind == CardKind::OffworldOffice && matches!(c.zone, Zone::Root(_)))
                    .ok_or("No Office to access")?
                    .id,
            }
        } else {
            Pending::Paid {
                priority: position.active,
                passed: false,
                window: TurnWindow::Start,
            }
        };
        let state = State {
            corp_credits: position.corp_credits,
            revision: 0,
            cards,
            hq_positions: vec![],
            run: None,
            run_result: None,
            pending: Some(pending),
            history: vec![],
            turn: Some(Turn {
                active: position.active,
                number: 2,
                clicks,
                runner_credits: position.runner_credits,
                points: position.agenda_points,
                winner: None,
                terminal: None,
                selected: vec![],
                random_state: random.state(),
                random_algorithm: "unfolded-tabletop-splitmix64-v1".into(),
                next_remote,
                run: None,
                hq_breached: false,
            }),
        };
        if !state.valid() {
            return Err("Invalid engineering position");
        }
        Ok(Self(state))
    }
    pub(super) fn observe_turns(&self, viewer: Player) -> Observation {
        let state = &self.0;
        let turn = state.turn.as_ref().unwrap();
        let views = |zone| {
            state
                .cards
                .iter()
                .filter(|c| c.zone == zone)
                .map(|c| state.turn_view(c, viewer))
                .collect()
        };
        let servers = state
            .servers()
            .into_iter()
            .map(|server| ServerView {
                server,
                root: views(Zone::Root(server)),
                ice: state
                    .cards
                    .iter()
                    .filter(|c| state.card_server(c) == Some(server) && c.kind.ice())
                    .map(|c| state.turn_view(c, viewer))
                    .collect(),
            })
            .collect();
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
            run: state.run_view(viewer),
            decision: state
                .pending
                .filter(|p| p.actor() == viewer)
                .map(|_| Decision {
                    actor: viewer,
                    revision: state.revision,
                    choices: state.turn_choices(),
                }),
            corp_credits: state.corp_credits,
            runner_credits: turn.runner_credits,
            clicks: turn.clicks,
            agenda_points: turn.points,
            active_player: Some(turn.active),
            turn_number: turn.number,
            run_result: state.run_result,
            match_winner: turn.winner,
            hq_count: state.count(Zone::Hq),
            rd_count: state.count(Zone::Rd),
            stack_count: state.count(Zone::Stack),
            grip_count: state.count(Zone::Grip),
            ice: views(Zone::ProtectingHq),
            archives: views(Zone::Archives),
            approached_ice: None,
            hand: views(if viewer == Player::Corp {
                Zone::Hq
            } else {
                Zone::Grip
            }),
            installing: state
                .cards
                .iter()
                .find(|c| c.zone == Zone::Installing)
                .map(|c| state.turn_view(c, viewer)),
            rig: views(Zone::Rig),
            heap: views(Zone::Heap),
            play_area: views(Zone::PlayArea),
            corp_score: views(Zone::CorpScore),
            runner_score: views(Zone::RunnerScore),
            servers,
            memory_used: state.memory_used(),
            history: state
                .history
                .iter()
                .map(|h| h[viewer.index()].clone())
                .collect(),
            terminal_reason: turn.terminal.map(|t| match t {
                Terminal::AgendaPoints => "Agenda victory",
                Terminal::CorpDrawFailure => "Corp could not draw from R&D",
            }),
            install_destination: match state.pending {
                Some(Pending::InstallTrash { server, .. }) => Some(server),
                _ => None,
            },
            install_cost: match state.pending {
                Some(Pending::InstallTrash { card, server, .. }) => {
                    Some(state.install_cost(card, server))
                }
                _ => None,
            },
            accessed: state
                .accessed_in_run(viewer)
                .or_else(|| match state.pending {
                    Some(Pending::AccessOffice { card }) if viewer == Player::Runner => {
                        let card = state.card(card);
                        let (title, printing, text) = card.kind.description();
                        Some(CardView::face(title, printing, text, true, false))
                    }
                    _ => None,
                }),
            phase: match state.pending {
                Some(Pending::RunServer) => "Overclock · choose server",
                Some(Pending::RunStep { .. }) => state.run_phase(),
                Some(Pending::Paid {
                    window: TurnWindow::Start,
                    ..
                }) if turn.active == Player::Runner => "Turn start · paid / rez window",
                Some(Pending::Paid {
                    window: TurnWindow::BeforeAction,
                    ..
                }) if turn.active == Player::Runner => "Paid / rez window",
                Some(Pending::Paid {
                    window: TurnWindow::Start,
                    ..
                }) => "Turn start · paid / rez / scoring window",
                Some(Pending::Paid {
                    window: TurnWindow::BeforeAction,
                    ..
                }) => "Paid / rez / scoring window",
                Some(Pending::Paid {
                    window: TurnWindow::End,
                    ..
                }) => "Turn end · paid / rez window",
                Some(Pending::Action { .. }) => "Action phase",
                Some(Pending::Discard { .. }) => "Choose cards to discard simultaneously",
                Some(Pending::InstallDestination { .. }) => "Choose installation destination",
                Some(Pending::InstallTrash { .. }) => "Installation · optional trash before paying",
                Some(Pending::SeamlessTarget) => "Seamless Launch · choose one installed card",
                Some(Pending::AccessOffice { .. }) => "Accessing Offworld Office · must steal",
                None => "Match complete",
                _ => "Run",
            },
        }
    }
    pub(super) fn choose_turns(
        &mut self,
        actor: Player,
        action: Action,
    ) -> Result<(), &'static str> {
        let mut candidate = self.0.clone();
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or("Decision revision exhausted")?;
        candidate.apply_turn_choice(actor, action)?;
        if candidate
            .cards
            .iter()
            .any(|c| c.kind.unsupported() && !matches!(c.zone, Zone::Rd | Zone::Stack))
        {
            return Err(
                "This engineering continuation cannot make an unimplemented card live. The choice and save are unchanged.",
            );
        }
        if !candidate.valid() {
            return Err("Unsupported transition state");
        }
        self.0 = candidate;
        Ok(())
    }
}
impl State {
    fn turn(&self) -> &Turn {
        self.turn.as_ref().unwrap()
    }
    fn turn_mut(&mut self) -> &mut Turn {
        self.turn.as_mut().unwrap()
    }
    fn count(&self, zone: Zone) -> usize {
        self.cards.iter().filter(|c| c.zone == zone).count()
    }
    fn memory_used(&self) -> u8 {
        self.cards
            .iter()
            .filter(|c| c.zone == Zone::Rig)
            .map(|c| c.kind.memory())
            .sum()
    }
    fn credits(&self, player: Player) -> u32 {
        if player == Player::Corp {
            self.corp_credits
        } else {
            self.turn().runner_credits
        }
    }
    fn pay(&mut self, player: Player, amount: u32) {
        if player == Player::Corp {
            self.corp_credits -= amount;
        } else {
            self.turn_mut().runner_credits -= amount;
        }
    }
    fn gain(&mut self, player: Player, amount: u32) {
        if player == Player::Corp {
            self.corp_credits += amount;
        } else {
            self.turn_mut().runner_credits += amount;
        }
    }
    fn installed(card: &CardInstance) -> bool {
        matches!(
            card.zone,
            Zone::Root(_) | Zone::Protecting(_) | Zone::ProtectingHq | Zone::Rig
        )
    }
    fn seamless_targets(&self) -> Vec<CardId> {
        self.cards
            .iter()
            .filter(|c| {
                Self::installed(c)
                    && !(c.installed_by == Some(Player::Corp)
                        && c.installed_turn == Some(self.turn().number))
            })
            .map(|c| c.id)
            .collect()
    }
    fn finish_action(&mut self) {
        if self.turn().winner.is_none() {
            self.paid(TurnWindow::BeforeAction);
        }
    }
    fn trash(&mut self, id: CardId) {
        let card = self.card_mut(id);
        card.faceup |=
            card.rezzed || card.kind.owner() == Player::Runner || card.zone == Zone::PlayArea;
        card.zone = if card.kind.owner() == Player::Corp {
            Zone::Archives
        } else {
            Zone::Heap
        };
        card.rezzed = false;
        card.advancement = 0;
        card.installed_turn = None;
        card.installed_by = None;
    }
    fn complete_play(&mut self) {
        if let Some(card) = self.cards.iter().find(|c| c.zone == Zone::PlayArea) {
            self.trash(card.id);
        }
        self.finish_action();
    }
    fn win_checkpoint(&mut self) {
        for player in [self.turn().active, self.turn().active.other()] {
            if self.turn().points[player.index()] >= 6 {
                self.turn_mut().winner = Some(player);
                self.turn_mut().terminal = Some(Terminal::AgendaPoints);
                self.pending = None;
                self.log(format!("{player:?} reaches 6 agenda points and wins."));
                break;
            }
        }
    }
    fn card_server(&self, card: &CardInstance) -> Option<Server> {
        match card.zone {
            Zone::ProtectingHq => Some(Server::Hq),
            Zone::Root(server) | Zone::Protecting(server) => Some(server),
            _ => None,
        }
    }
    fn servers(&self) -> Vec<Server> {
        let mut servers = vec![Server::Hq, Server::Rd, Server::Archives];
        for card in &self.cards {
            if let Some(server) = self.card_server(card)
                && !servers.contains(&server)
            {
                servers.push(server);
            }
        }
        if let Some(Pending::InstallTrash {
            server: server @ Server::Remote(_),
            ..
        }) = self.pending
            && !servers.contains(&server)
        {
            servers.push(server);
        }
        servers
    }
    fn turn_view(&self, card: &CardInstance, viewer: Player) -> CardView {
        let visible = self.run_knows(card.id, viewer)
            || match card.zone {
                Zone::Hq => viewer == Player::Corp,
                Zone::Grip => viewer == Player::Runner,
                Zone::Rd | Zone::Stack => false,
                Zone::Archives => viewer == Player::Corp || card.faceup,
                Zone::Root(_) | Zone::Protecting(_) | Zone::ProtectingHq => {
                    viewer == Player::Corp || card.rezzed
                }
                Zone::Installing => viewer == card.kind.owner(),
                _ => true,
            };
        let (title, printing, text) = card.kind.description();
        let mut view = CardView::face(title, printing, text, visible, card.rezzed);
        view.instance = visible.then_some(card.id);
        view.face_up = match card.zone {
            Zone::Archives => card.faceup,
            Zone::Root(_) | Zone::Protecting(_) | Zone::ProtectingHq => card.rezzed,
            Zone::Installing => false,
            _ => visible,
        };
        view.advancement = card.advancement;
        if card.kind == CardKind::Overclock && card.zone == Zone::PlayArea {
            view.hosted_credits = self.turn().run.as_ref().map_or(5, |r| r.hosted);
        }
        view.selected = self.turn().selected.contains(&card.id);
        view
    }
    fn log(&mut self, public: String) {
        self.history.push([public.clone(), public]);
    }
    fn paid(&mut self, window: TurnWindow) {
        self.pending = Some(Pending::Paid {
            priority: self.turn().active,
            passed: false,
            window,
        });
    }
    fn draw(&mut self, player: Player) {
        let (deck, hand) = if player == Player::Corp {
            (Zone::Rd, Zone::Hq)
        } else {
            (Zone::Stack, Zone::Grip)
        };
        if let Some(card) = self.cards.iter_mut().find(|c| c.zone == deck) {
            card.zone = hand;
            self.log(format!("{player:?} draws a card."));
        } else if player == Player::Corp {
            self.turn_mut().winner = Some(Player::Runner);
            self.turn_mut().terminal = Some(Terminal::CorpDrawFailure);
            self.pending = None;
            self.log("Corp cannot draw from R&D. Runner wins.".into());
        }
    }
    pub(super) fn valid_turns(&self) -> bool {
        let turn = self.turn();
        if turn.random_algorithm != "unfolded-tabletop-splitmix64-v1"
            || self.run.is_some()
            || !self.hq_positions.is_empty()
            || self.revision > u64::MAX - 100
            || turn.number == u64::MAX
            || self.cards.len() > 66
        {
            return false;
        }
        if self.corp_credits > u32::MAX - 100
            || turn.runner_credits > u32::MAX - 100
            || turn.points.iter().any(|&p| p > 7)
            || turn.next_remote == u32::MAX
            || turn.clicks[0] > 3
            || turn.clicks[1] > 4
        {
            return false;
        }
        for (index, card) in self.cards.iter().enumerate() {
            if self.cards[..index].iter().any(|other| other.id == card.id)
                || card.advancement > u32::MAX - 2
            {
                return false;
            }
            // The exact engineering deck contains these identities, not their
            // effects. Refuse before committing a draw or any live placement.
            if card.kind.unsupported() && !matches!(card.zone, Zone::Rd | Zone::Stack) {
                return false;
            }
            let owner = card.kind.owner();
            let zone_valid = match card.zone {
                Zone::Hq | Zone::Rd | Zone::Archives => owner == Player::Corp,
                Zone::Grip | Zone::Stack | Zone::Heap => owner == Player::Runner,
                Zone::Rig => owner == Player::Runner && card.kind.installable(),
                Zone::CorpScore | Zone::RunnerScore => card.kind == CardKind::OffworldOffice,
                Zone::Root(Server::Remote(_)) => matches!(
                    card.kind,
                    CardKind::OffworldOffice | CardKind::ManegarmSkunkworks
                ),
                Zone::Root(Server::Hq | Server::Rd | Server::Archives) => {
                    card.kind == CardKind::ManegarmSkunkworks
                }
                Zone::ProtectingHq
                | Zone::Protecting(
                    Server::Hq | Server::Rd | Server::Archives | Server::Remote(_),
                ) => card.kind.ice(),
                Zone::Installing => card.kind.installable(),
                Zone::PlayArea => card.kind.play_cost().is_some(),
                _ => false,
            };
            if !zone_valid
                || (card.rezzed
                    && !((card.kind.ice() || card.kind == CardKind::ManegarmSkunkworks)
                        && Self::installed(card)))
                || (Self::installed(card)
                    && (card.installed_turn.is_none()
                        || card.installed_turn > Some(turn.number)
                        || card.installed_by != Some(owner)))
            {
                return false;
            }
            if let Some(Server::Remote(id)) = self.card_server(card)
                && (id == 0 || id >= turn.next_remote)
            {
                return false;
            }
        }
        if self.memory_used() > 4 {
            return false;
        }
        if let Some(winner) = turn.winner {
            return self.pending.is_none()
                && match turn.terminal {
                    Some(Terminal::AgendaPoints) => turn.points[winner.index()] >= 6,
                    Some(Terminal::CorpDrawFailure) => {
                        turn.winner == Some(Player::Runner) && self.count(Zone::Rd) == 0
                    }
                    None => false,
                };
        }
        if turn.terminal.is_some() || turn.points.iter().any(|&points| points >= 6) {
            return false;
        }
        let Some(pending) = self.pending else {
            return false;
        };
        let installing = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::Installing)
            .count();
        let play_area = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::PlayArea)
            .count();
        if !matches!(pending, Pending::Discard { .. }) && !turn.selected.is_empty() {
            return false;
        }
        if installing
            != usize::from(
                matches!(
                    pending,
                    Pending::InstallDestination { .. } | Pending::InstallTrash { .. }
                ) || self.run_installing(),
            )
            || play_area
                != usize::from(
                    matches!(pending, Pending::SeamlessTarget | Pending::RunServer)
                        || turn.run.as_ref().is_some_and(|r| r.overclock.is_some()),
                )
        {
            return false;
        }
        if matches!(pending, Pending::RunServer | Pending::RunStep { .. }) {
            return self.valid_run();
        }
        if turn.run.is_some() {
            return false;
        }
        match pending {
            Pending::Paid {
                priority,
                passed,
                window,
            } => {
                let _ = (priority, passed);
                window != TurnWindow::End || turn.clicks[turn.active.index()] == 0
            }
            Pending::Action { actor } => actor == turn.active && turn.clicks[actor.index()] > 0,
            Pending::Discard { actor } => {
                let zone = if actor == Player::Corp {
                    Zone::Hq
                } else {
                    Zone::Grip
                };
                actor == turn.active
                    && turn.clicks[actor.index()] == 0
                    && self.count(zone) > 5
                    && turn.selected.iter().enumerate().all(|(i, id)| {
                        !turn.selected[..i].contains(id)
                            && self.cards.iter().any(|c| c.id == *id && c.zone == zone)
                    })
            }
            Pending::InstallDestination { actor, card } => {
                actor == Player::Corp
                    && actor == turn.active
                    && self.cards.iter().any(|c| {
                        c.id == card && c.zone == Zone::Installing && c.kind.owner() == actor
                    })
            }
            Pending::InstallTrash {
                actor,
                card,
                server,
            } => {
                actor == turn.active
                    && self.cards.iter().any(|c| {
                        c.id == card
                            && c.zone == Zone::Installing
                            && c.kind.owner() == actor
                            && (if actor == Player::Runner {
                                server == Server::Rig
                            } else {
                                matches!(
                                    server,
                                    Server::Hq | Server::Rd | Server::Archives | Server::Remote(_)
                                ) && (c.kind.ice()
                                    || c.kind == CardKind::ManegarmSkunkworks
                                    || matches!(server, Server::Remote(_)))
                            })
                    })
            }
            Pending::SeamlessTarget => {
                turn.active == Player::Corp
                    && self
                        .cards
                        .iter()
                        .any(|c| c.zone == Zone::PlayArea && c.kind == CardKind::SeamlessLaunch)
                    && !self.seamless_targets().is_empty()
            }
            Pending::AccessOffice { card } => {
                turn.active == Player::Runner
                    && self.cards.iter().any(|c| {
                        c.id == card
                            && c.kind == CardKind::OffworldOffice
                            && matches!(c.zone, Zone::Root(_))
                    })
            }
            _ => false,
        }
    }
    fn turn_choices(&self) -> Vec<Action> {
        if matches!(
            self.pending,
            Some(Pending::RunServer | Pending::RunStep { .. })
        ) {
            return self.run_choices();
        }
        match self.pending {
            Some(Pending::Paid {
                priority, window, ..
            }) => {
                let mut choices = vec![];
                if priority == Player::Corp
                    && self.turn().active == Player::Corp
                    && window != TurnWindow::End
                {
                    choices.extend(
                        self.cards
                            .iter()
                            .filter(|c| {
                                matches!(c.zone, Zone::Root(_))
                                    && c.kind == CardKind::OffworldOffice
                                    && c.advancement >= 4
                            })
                            .map(|c| Action::Score(c.id)),
                    );
                    // ICE can only rez when approached. The admitted rig has
                    // no paid abilities applicable outside a run.
                }
                if priority == Player::Corp {
                    choices.extend(self.non_ice_rez_choices());
                }
                if priority == Player::Runner {
                    choices.extend(self.boost_choices(0));
                }
                choices.push(Action::PassPriority);
                choices
            }
            Some(Pending::Action { actor }) => {
                let mut choices = vec![Action::GainCredit, Action::Draw];
                if actor == Player::Runner {
                    choices.extend(self.servers().into_iter().map(Action::RunServer));
                }
                if actor == Player::Corp && self.turn().clicks[0] >= 3 {
                    choices.push(Action::Purge);
                }
                for card in &self.cards {
                    if card.zone
                        == if actor == Player::Corp {
                            Zone::Hq
                        } else {
                            Zone::Grip
                        }
                    {
                        if let Some(cost) = card.kind.play_cost()
                            && cost <= self.credits(actor)
                        {
                            choices.push(Action::PlayCard(card.id));
                        }
                        if card.kind.installable()
                            && card.kind.install_cost() <= self.credits(actor)
                        {
                            choices.push(Action::InstallCard(card.id));
                        }
                    }
                    if actor == Player::Corp
                        && self.corp_credits > 0
                        && card.kind == CardKind::OffworldOffice
                        && matches!(card.zone, Zone::Root(_))
                    {
                        choices.push(Action::Advance(card.id));
                    }
                }
                choices
            }
            Some(Pending::SeamlessTarget) => self
                .seamless_targets()
                .into_iter()
                .map(Action::SelectTarget)
                .collect(),
            Some(Pending::AccessOffice { .. }) => vec![Action::StealAgenda],
            Some(Pending::Discard { actor }) => {
                let zone = if actor == Player::Corp {
                    Zone::Hq
                } else {
                    Zone::Grip
                };
                let mut choices: Vec<_> = self
                    .cards
                    .iter()
                    .filter(|c| c.zone == zone)
                    .map(|c| Action::ToggleDiscard(c.id))
                    .collect();
                if self.turn().selected.len() == self.count(zone).saturating_sub(5) {
                    choices.push(Action::ConfirmDiscard);
                }
                choices
            }
            Some(Pending::InstallDestination { card, .. }) => {
                let mut servers = self.servers();
                if !self.card(card).kind.ice()
                    && self.card(card).kind != CardKind::ManegarmSkunkworks
                {
                    servers.retain(|s| matches!(s, Server::Remote(_)));
                }
                servers.push(Server::NewRemote);
                servers.into_iter().map(Action::Destination).collect()
            }
            Some(Pending::InstallTrash {
                actor,
                card,
                server,
            }) => {
                let kind = self.card(card).kind;
                let mut choices: Vec<_> = self
                    .cards
                    .iter()
                    .filter(|c| {
                        if actor == Player::Runner {
                            kind.memory() > 0 && c.zone == Zone::Rig && c.kind.memory() > 0
                        } else if kind.ice() {
                            c.kind.ice() && self.card_server(c) == Some(server)
                        } else {
                            c.zone == Zone::Root(server)
                        }
                    })
                    .map(|c| Action::TrashInstalled(c.id))
                    .collect();
                let cost = self.install_cost(card, server);
                let fits = if actor == Player::Runner {
                    self.memory_used() + kind.memory() <= 4
                } else {
                    kind.ice()
                        || kind == CardKind::ManegarmSkunkworks
                        || !self.cards.iter().any(|c| {
                            c.zone == Zone::Root(server) && c.kind == CardKind::OffworldOffice
                        })
                };
                if fits && cost <= self.credits(actor) {
                    choices.push(Action::FinishInstall);
                }
                choices
            }
            _ => vec![],
        }
    }
    fn install_cost(&self, id: CardId, server: Server) -> u32 {
        let kind = self.card(id).kind;
        if kind.ice() {
            self.cards
                .iter()
                .filter(|c| c.kind.ice() && self.card_server(c) == Some(server))
                .count() as u32
        } else {
            kind.install_cost()
        }
    }
    fn apply_turn_choice(&mut self, actor: Player, action: Action) -> Result<(), &'static str> {
        if matches!(action, Action::RunServer(_))
            || matches!(
                self.pending,
                Some(Pending::RunServer | Pending::RunStep { .. })
            )
        {
            return self.apply_run_choice(actor, action);
        }
        match (self.pending.unwrap(), action) {
            (Pending::Paid { window, .. }, Action::Rez(id)) => {
                self.rez_non_ice(id);
                self.pending = Some(Pending::Paid {
                    priority: actor,
                    passed: false,
                    window,
                });
            }
            (Pending::Paid { window, .. }, Action::Boost(id)) => {
                let cost = if self.card(id).kind == CardKind::Cleaver {
                    2
                } else {
                    1
                };
                self.pay(Player::Runner, cost);
                self.log(format!("Runner pays {cost} pool credits to boost {}; outside an encounter the boost expires at the next checkpoint.",self.card(id).kind.description().0));
                self.pending = Some(Pending::Paid {
                    priority: actor,
                    passed: false,
                    window,
                });
            }
            (
                Pending::Paid {
                    passed: false,
                    window,
                    ..
                },
                Action::PassPriority,
            ) => {
                self.pending = Some(Pending::Paid {
                    priority: actor.other(),
                    passed: true,
                    window,
                })
            }
            (
                Pending::Paid {
                    passed: true,
                    window,
                    ..
                },
                Action::PassPriority,
            ) => match window {
                TurnWindow::Start => {
                    if self.turn().active == Player::Corp {
                        self.draw(Player::Corp);
                    }
                    if self.turn().winner.is_none() {
                        self.paid(TurnWindow::BeforeAction);
                    }
                }
                TurnWindow::BeforeAction => {
                    let active = self.turn().active;
                    if self.turn().clicks[active.index()] > 0 {
                        self.pending = Some(Pending::Action { actor: active });
                    } else if self.count(if active == Player::Corp {
                        Zone::Hq
                    } else {
                        Zone::Grip
                    }) > 5
                    {
                        self.turn_mut().selected.clear();
                        self.pending = Some(Pending::Discard { actor: active });
                    } else {
                        self.paid(TurnWindow::End);
                    }
                }
                TurnWindow::End => {
                    let active = self.turn().active.other();
                    let turn = self.turn_mut();
                    turn.active = active;
                    turn.number += 1;
                    turn.hq_breached = false;
                    turn.clicks = [0, 0];
                    turn.clicks[active.index()] = if active == Player::Corp { 3 } else { 4 };
                    self.paid(TurnWindow::Start);
                    self.log(format!("{active:?} turn begins."));
                }
            },
            (Pending::Action { .. }, Action::GainCredit | Action::Draw) => {
                self.turn_mut().clicks[actor.index()] -= 1;
                if action == Action::Draw {
                    self.draw(actor);
                } else {
                    self.gain(actor, 1);
                    self.log(format!("{actor:?} spends 1 click to gain 1 credit."));
                }
                self.finish_action();
            }
            (Pending::Action { .. }, Action::Purge) => {
                self.turn_mut().clicks[0] -= 3;
                self.log("Corp spends 3 clicks to purge virus counters.".into());
                self.finish_action();
            }
            (Pending::Action { .. }, Action::PlayCard(id)) => {
                let kind = self.card(id).kind;
                self.pay(actor, kind.play_cost().unwrap());
                self.turn_mut().clicks[actor.index()] -= 1;
                self.card_mut(id).zone = Zone::PlayArea;
                self.log(format!("{actor:?} plays {}.", kind.description().0));
                match kind {
                    CardKind::Overclock => {
                        self.pending = Some(Pending::RunServer);
                        return Ok(());
                    }
                    CardKind::SureGamble | CardKind::HedgeFund => self.gain(actor, 9),
                    CardKind::GovernmentSubsidy => self.gain(actor, 15),
                    CardKind::SeamlessLaunch if !self.seamless_targets().is_empty() => {
                        self.pending = Some(Pending::SeamlessTarget);
                        return Ok(());
                    }
                    CardKind::SeamlessLaunch => {}
                    _ => unreachable!("legal play"),
                }
                self.complete_play();
            }
            (Pending::SeamlessTarget, Action::SelectTarget(id)) => {
                self.card_mut(id).advancement += 2;
                self.log("Seamless Launch places 2 advancement counters on the selected card; this is not advancing.".into());
                self.complete_play();
            }
            (Pending::Action { .. }, Action::Advance(id)) => {
                self.pay(actor, 1);
                self.turn_mut().clicks[0] -= 1;
                self.card_mut(id).advancement += 1;
                self.log("Corp advances an installed card: 1 click and 1 credit.".into());
                self.finish_action();
            }
            (Pending::Paid { window, .. }, Action::Score(id)) => {
                let card = self.card_mut(id);
                card.zone = Zone::CorpScore;
                card.advancement = 0;
                card.faceup = true;
                card.installed_turn = None;
                card.installed_by = None;
                self.turn_mut().points[0] += 2;
                self.log("Corp scores Offworld Office for 2 agenda points.".into());
                self.win_checkpoint();
                if self.turn().winner.is_none() {
                    self.gain(Player::Corp, 7);
                    self.log("Offworld Office: Corp gains 7 credits.".into());
                    self.pending = Some(Pending::Paid {
                        priority: actor,
                        passed: false,
                        window,
                    });
                }
            }
            (Pending::AccessOffice { card }, Action::StealAgenda) => {
                let card = self.card_mut(card);
                card.zone = Zone::RunnerScore;
                card.advancement = 0;
                card.faceup = true;
                card.installed_turn = None;
                card.installed_by = None;
                self.turn_mut().points[1] += 2;
                self.log("Runner steals Offworld Office for 2 agenda points. Its scoring reward does not trigger.".into());
                self.win_checkpoint();
                if self.turn().winner.is_none() {
                    self.run_result = Some(RunResult::SuccessfulAccess);
                    self.finish_action();
                }
            }
            (Pending::Discard { .. }, Action::ToggleDiscard(id)) => {
                let selected = &mut self.turn_mut().selected;
                if let Some(index) = selected.iter().position(|&c| c == id) {
                    selected.remove(index);
                } else {
                    selected.push(id);
                }
            }
            (Pending::Discard { actor }, Action::ConfirmDiscard) => {
                let selected = std::mem::take(&mut self.turn_mut().selected);
                for id in &selected {
                    let card = self.card_mut(*id);
                    card.zone = if actor == Player::Corp {
                        Zone::Archives
                    } else {
                        Zone::Heap
                    };
                    card.faceup = actor == Player::Runner;
                }
                self.log(format!(
                    "{actor:?} discards {} cards simultaneously.",
                    selected.len()
                ));
                self.paid(TurnWindow::End);
            }
            (Pending::Action { .. }, Action::InstallCard(id)) => {
                self.turn_mut().clicks[actor.index()] -= 1;
                self.card_mut(id).zone = Zone::Installing;
                self.pending = if actor == Player::Corp {
                    Some(Pending::InstallDestination { actor, card: id })
                } else {
                    Some(Pending::InstallTrash {
                        actor,
                        card: id,
                        server: Server::Rig,
                    })
                };
                self.history.push(if actor == Player::Corp {
                    [
                        format!(
                            "Corp begins installing {}.",
                            self.card(id).kind.description().0
                        ),
                        "Corp begins installing a facedown card.".into(),
                    ]
                } else {
                    [
                        "Runner begins an installation.".into(),
                        "Runner begins an installation.".into(),
                    ]
                });
            }
            (Pending::InstallDestination { actor, card }, Action::Destination(mut server)) => {
                if server == Server::NewRemote {
                    server = Server::Remote(self.turn().next_remote);
                    self.turn_mut().next_remote += 1;
                }
                self.pending = Some(Pending::InstallTrash {
                    actor,
                    card,
                    server,
                });
            }
            (Pending::InstallTrash { .. }, Action::TrashInstalled(id)) => {
                self.trash(id);
                self.log(format!("{actor:?} trashes a card during installation."));
            }
            (
                Pending::InstallTrash {
                    actor,
                    card,
                    server,
                },
                Action::FinishInstall,
            ) => {
                let kind = self.card(card).kind;
                let cost = self.install_cost(card, server);
                self.pay(actor, cost);
                let number = self.turn().number;
                let installed = self.card_mut(card);
                installed.zone = if actor == Player::Runner {
                    Zone::Rig
                } else if kind.ice() {
                    Zone::Protecting(server)
                } else {
                    Zone::Root(server)
                };
                installed.rezzed = false;
                installed.faceup = actor == Player::Runner;
                installed.installed_turn = Some(number);
                installed.installed_by = Some(actor);
                installed.advancement = 0;
                // Ordered cards are innermost first; moving this instance to
                // the end places newly installed ICE outermost (CR 8.5.4).
                let index = self.cards.iter().position(|c| c.id == card).unwrap();
                let installed = self.cards.remove(index);
                self.cards.push(installed);
                if kind == CardKind::DocklandsPass {
                    let older: Vec<_> = self
                        .cards
                        .iter()
                        .filter(|c| c.id != card && c.zone == Zone::Rig && c.kind == kind)
                        .map(|c| c.id)
                        .collect();
                    for id in older {
                        self.trash(id);
                    }
                }
                self.log(format!(
                    "{actor:?} completes installation, paying {cost} credits."
                ));
                self.finish_action();
            }
            _ => return Err("Unsupported turn choice"),
        }
        Ok(())
    }
}
