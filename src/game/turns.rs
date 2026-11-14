//! Ordinary turn timing in the same match. CR 26.03 chapters 5, 8.5, 9.2 and
//! 10.3; card text from the pinned official snapshot. Engineering positions do
//! not admit complete introductory decks or expose an additional playable mode.
use super::*;
use unfolded_tabletop::random::SeededRandom;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Server { #[default] Hq, Rd, Archives, Remote(u32), NewRemote, Rig }
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ServerView { pub server: Server, pub root: Vec<CardView>, pub ice: Vec<CardView> }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnWindow { Start, BeforeAction, End }
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
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) enum Terminal { AgendaPoints, CorpDrawFailure }
impl Player {
    pub fn other(self) -> Self { match self { Self::Corp => Self::Runner, Self::Runner => Self::Corp } }
    pub fn index(self) -> usize { usize::from(self == Self::Runner) }
}
impl CardKind {
    pub fn description(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Bran => ("Brân 1.0", "30039", BRAN_TEXT),
            Self::Palisade => ("Palisade", "30072", PALISADE_TEXT),
            Self::SureGamble => ("Sure Gamble", "30030", "Gain 9 credits. Play cost: 5 credits."),
            Self::HedgeFund => ("Hedge Fund", "30075", "Gain 9 credits. Play cost: 5 credits."),
            Self::GovernmentSubsidy => ("Government Subsidy", "30064", "Gain 15 credits. Play cost: 10 credits."),
            Self::OffworldOffice => ("Offworld Office", "30067", "When you score this agenda, gain 7 credits. Advancement requirement: 4. Agenda points: 2."),
            Self::SeamlessLaunch => ("Seamless Launch", "30040", "Place 2 advancement counters on 1 installed card that you did not install this turn. Play cost: 1 credit."),
            Self::Cleaver => ("Cleaver", "30006", "1 credit: Break up to 2 barrier subroutines. 2 credits: +3 strength. Install: 3 credits. Memory: 2."),
            Self::Unity => ("Unity", "30026", "1 credit: Break 1 code gate subroutine. 1 credit: +1 strength for each installed icebreaker. Install: 3 credits. Memory: 1."),
            Self::DocklandsPass => ("Docklands Pass", "30013", "The first time each turn you breach HQ, access 1 additional card. Unique. Install: 2 credits."),
            #[cfg(test)]
            Self::ConcealedFixture => ("Unrevealed fixture", "fixture", "Private fixture text"),
        }
    }
    fn owner(self) -> Player { if matches!(self, Self::SureGamble | Self::Cleaver | Self::Unity | Self::DocklandsPass) { Player::Runner } else { Player::Corp } }
    fn memory(self) -> u8 { match self { Self::Cleaver => 2, Self::Unity => 1, _ => 0 } }
    fn ice(self) -> bool { matches!(self, Self::Bran | Self::Palisade) }
    fn installable(self) -> bool { self.ice() || matches!(self, Self::OffworldOffice | Self::Cleaver | Self::Unity | Self::DocklandsPass) }
    fn play_cost(self) -> Option<u32> { match self { Self::SureGamble | Self::HedgeFund => Some(5), Self::GovernmentSubsidy => Some(10), Self::SeamlessLaunch => Some(1), _ => None } }
    fn install_cost(self) -> u32 { match self { Self::Cleaver | Self::Unity => 3, Self::DocklandsPass => 2, _ => 0 } }
}

/// Explicit source-derived engineering input. No ordinary UI can create one;
/// production loading and every subsequent choice use the normal match path.
#[derive(Clone)]
pub struct EngineeringCard { pub kind: CardKind, pub zone: Zone, pub advancement: u32, pub installed_turn: Option<u64>, pub rezzed: bool }
impl EngineeringCard {
    pub fn new(kind: CardKind, zone: Zone) -> Self { Self { kind, zone, advancement: 0, installed_turn: None, rezzed: false } }
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
}
impl EngineeringPosition {
    pub fn economy(seed: u64) -> Self {
        use CardKind::*;
        Self { active: Player::Corp, corp_credits: 5, runner_credits: 5, agenda_points: [0, 0], seed, shuffle_decks: true,
            cards: vec![EngineeringCard::new(HedgeFund, Zone::Hq), EngineeringCard::new(OffworldOffice, Zone::Hq), EngineeringCard::new(SeamlessLaunch, Zone::Rd), EngineeringCard::new(GovernmentSubsidy, Zone::Rd), EngineeringCard::new(HedgeFund, Zone::Rd), EngineeringCard::new(SureGamble, Zone::Grip), EngineeringCard::new(Cleaver, Zone::Grip), EngineeringCard::new(Unity, Zone::Grip), EngineeringCard::new(DocklandsPass, Zone::Grip), EngineeringCard::new(SureGamble, Zone::Stack)] }
    }
}
impl Match {
    pub fn engineering_turns(position: EngineeringPosition) -> Result<Self, &'static str> {
        let mut cards: Vec<_> = position.cards.into_iter().enumerate().map(|(i, input)| {
            let mut card = CardInstance::new(CardId(i as u32 + 1), input.kind, input.zone, input.rezzed);
            card.advancement = input.advancement;
            card.installed_turn = input.installed_turn;
            card.installed_by = input.installed_turn.map(|_| input.kind.owner());
            card.faceup = input.rezzed || matches!(input.zone, Zone::Heap | Zone::CorpScore | Zone::RunnerScore | Zone::Rig);
            card
        }).collect();
        let mut random = SeededRandom::new(position.seed);
        if position.shuffle_decks {
            for zone in [Zone::Rd, Zone::Stack] {
                let indices: Vec<_> = cards.iter().enumerate().filter_map(|(i, c)| (c.zone == zone).then_some(i)).collect();
                let mut shuffled: Vec<_> = indices.iter().map(|&i| cards[i].clone()).collect();
                random.shuffle(&mut shuffled);
                for (index, card) in indices.into_iter().zip(shuffled) { cards[index] = card; }
            }
        }
        let next_remote = cards.iter().filter_map(|c| match c.zone { Zone::Root(Server::Remote(id)) | Zone::Protecting(Server::Remote(id)) => Some(id), _ => None }).max().unwrap_or(0).checked_add(1).ok_or("Remote identifiers exhausted")?;
        let mut clicks = [0, 0]; clicks[position.active.index()] = if position.active == Player::Corp { 3 } else { 4 };
        let state = State { corp_credits: position.corp_credits, revision: 0, cards, hq_positions: vec![], run: None, run_result: None,
            pending: Some(Pending::Paid { priority: position.active, passed: false, window: TurnWindow::Start }), history: vec![],
            turn: Some(Turn { active: position.active, number: 2, clicks, runner_credits: position.runner_credits, points: position.agenda_points, winner: None, terminal: None, selected: vec![], random_state: random.state(), random_algorithm: "unfolded-tabletop-xorshift64-v1".into(), next_remote }) };
        if !state.valid() { return Err("Invalid engineering position"); }
        Ok(Self(state))
    }
    pub(super) fn observe_turns(&self, viewer: Player) -> Observation {
        let state = &self.0; let turn = state.turn.as_ref().unwrap();
        let views = |zone| state.cards.iter().filter(|c| c.zone == zone).map(|c| state.turn_view(c, viewer)).collect();
        let servers = state.servers().into_iter().map(|server| ServerView { server, root: views(Zone::Root(server)), ice: state.cards.iter().filter(|c| state.card_server(c) == Some(server) && c.kind.ice()).map(|c| state.turn_view(c, viewer)).collect() }).collect();
        Observation { corp_identity: CardView::face("The Syndicate", "30077", "Starter game only.", true, false), runner_identity: CardView::face("The Catalyst", "30076", "Starter game only.", true, false),
            decision: state.pending.filter(|p| p.actor() == viewer).map(|_| Decision { actor: viewer, revision: state.revision, choices: state.turn_choices() }),
            corp_credits: state.corp_credits, runner_credits: turn.runner_credits, clicks: turn.clicks, agenda_points: turn.points, active_player: Some(turn.active), turn_number: turn.number,
            run_result: state.run_result, match_winner: turn.winner, hq_count: state.count(Zone::Hq), rd_count: state.count(Zone::Rd), stack_count: state.count(Zone::Stack), grip_count: state.count(Zone::Grip),
            ice: views(Zone::ProtectingHq), archives: views(Zone::Archives), approached_ice: None,
            hand: views(if viewer == Player::Corp { Zone::Hq } else { Zone::Grip }), installing: state.cards.iter().find(|c| c.zone == Zone::Installing).map(|c| state.turn_view(c, viewer)), rig: views(Zone::Rig), heap: views(Zone::Heap), corp_score: views(Zone::CorpScore), runner_score: views(Zone::RunnerScore), servers, memory_used: state.memory_used(),
            history: state.history.iter().map(|h| h[viewer.index()].clone()).collect(), terminal_reason: turn.terminal.map(|t| match t { Terminal::AgendaPoints => "Agenda victory", Terminal::CorpDrawFailure => "Corp could not draw from R&D" }),
            phase: match state.pending { Some(Pending::Paid { window: TurnWindow::Start, .. }) => "Turn start · paid / rez / scoring window", Some(Pending::Paid { window: TurnWindow::BeforeAction, .. }) => "Paid / rez / scoring window", Some(Pending::Paid { window: TurnWindow::End, .. }) => "Turn end · paid / rez window", Some(Pending::Action { .. }) => "Action phase", Some(Pending::Discard { .. }) => "Choose cards to discard simultaneously", Some(Pending::InstallDestination { .. }) => "Choose installation destination", Some(Pending::InstallTrash { .. }) => "Installation · optional trash before paying", Some(Pending::SeamlessTarget) => "Seamless Launch · choose one installed card", None => "Match complete", _ => "Run" }, }
    }
    pub(super) fn choose_turns(&mut self, actor: Player, action: Action) -> Result<(), &'static str> {
        let mut candidate = self.0.clone();
        candidate.revision = candidate.revision.checked_add(1).ok_or("Decision revision exhausted")?;
        candidate.apply_turn_choice(actor, action)?;
        if !candidate.valid() { return Err("Unsupported transition state"); }
        self.0 = candidate; Ok(())
    }
}
impl State {
    fn turn(&self) -> &Turn { self.turn.as_ref().unwrap() }
    fn turn_mut(&mut self) -> &mut Turn { self.turn.as_mut().unwrap() }
    fn count(&self, zone: Zone) -> usize { self.cards.iter().filter(|c| c.zone == zone).count() }
    fn memory_used(&self) -> u8 { self.cards.iter().filter(|c| c.zone == Zone::Rig).map(|c| c.kind.memory()).sum() }
    fn card_server(&self, card: &CardInstance) -> Option<Server> { match card.zone { Zone::ProtectingHq => Some(Server::Hq), Zone::Root(server) | Zone::Protecting(server) => Some(server), _ => None } }
    fn servers(&self) -> Vec<Server> { let mut servers = vec![Server::Hq, Server::Rd, Server::Archives]; for card in &self.cards { if let Some(server) = self.card_server(card) && !servers.contains(&server) { servers.push(server); } } servers }
    fn turn_view(&self, card: &CardInstance, viewer: Player) -> CardView {
        let visible = match card.zone { Zone::Hq => viewer == Player::Corp, Zone::Grip => viewer == Player::Runner, Zone::Rd | Zone::Stack => false, Zone::Archives => card.faceup, Zone::Root(_) | Zone::Protecting(_) | Zone::ProtectingHq => viewer == Player::Corp || card.rezzed, Zone::Installing => viewer == card.kind.owner(), _ => true };
        let (title, printing, text) = card.kind.description();
        let mut view = CardView::face(title, printing, text, visible, card.rezzed);
        view.instance = visible.then_some(card.id); view.advancement = card.advancement; view.selected = self.turn().selected.contains(&card.id); view
    }
    fn log(&mut self, public: String) { self.history.push([public.clone(), public]); }
    fn paid(&mut self, window: TurnWindow) { self.pending = Some(Pending::Paid { priority: self.turn().active, passed: false, window }); }
    fn draw(&mut self, player: Player) {
        let (deck, hand) = if player == Player::Corp { (Zone::Rd, Zone::Hq) } else { (Zone::Stack, Zone::Grip) };
        if let Some(card) = self.cards.iter_mut().find(|c| c.zone == deck) { card.zone = hand; self.log(format!("{player:?} draws a card.")); }
        else if player == Player::Corp { self.turn_mut().winner = Some(Player::Runner); self.turn_mut().terminal = Some(Terminal::CorpDrawFailure); self.pending = None; self.log("Corp cannot draw from R&D. Runner wins.".into()); }
    }
    pub(super) fn valid_turns(&self) -> bool {
        let turn = self.turn();
        if turn.random_algorithm != "unfolded-tabletop-xorshift64-v1" || self.run.is_some() || !self.hq_positions.is_empty() || self.revision > u64::MAX - 100 || turn.number == u64::MAX || self.cards.len() > 66 { return false; }
        if self.cards.iter().enumerate().any(|(i,c)| self.cards[..i].iter().any(|p| p.id == c.id)) { return false; }
        if turn.winner.is_some() { return self.pending.is_none() && turn.terminal.is_some(); }
        self.pending.is_some()
    }
    fn turn_choices(&self) -> Vec<Action> {
        match self.pending {
            Some(Pending::Paid { .. }) => vec![Action::PassPriority],
            Some(Pending::Action { actor }) => { let mut choices = vec![Action::GainCredit]; if actor == Player::Corp || self.count(Zone::Stack) > 0 { choices.push(Action::Draw); } choices }
            _ => vec![],
        }
    }
    fn apply_turn_choice(&mut self, actor: Player, action: Action) -> Result<(), &'static str> {
        match (self.pending.unwrap(), action) {
            (Pending::Paid { passed: false, window, .. }, Action::PassPriority) => self.pending = Some(Pending::Paid { priority: actor.other(), passed: true, window }),
            (Pending::Paid { passed: true, window, .. }, Action::PassPriority) => match window {
                TurnWindow::Start => { if self.turn().active == Player::Corp { self.draw(Player::Corp); } if self.turn().winner.is_none() { self.paid(TurnWindow::BeforeAction); } },
                TurnWindow::BeforeAction => self.pending = Some(Pending::Action { actor: self.turn().active }),
                TurnWindow::End => return Err("Turn end not implemented"),
            },
            (Pending::Action { .. }, Action::GainCredit | Action::Draw) => {
                self.turn_mut().clicks[actor.index()] -= 1;
                if action == Action::Draw { self.draw(actor); } else { if actor == Player::Corp { self.corp_credits += 1; } else { self.turn_mut().runner_credits += 1; } self.log(format!("{actor:?} spends 1 click to gain 1 credit.")); }
                if self.turn().winner.is_none() { self.paid(TurnWindow::BeforeAction); }
            }
            _ => return Err("Unsupported turn choice"),
        } Ok(())
    }
}
