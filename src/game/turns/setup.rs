//! Exact System Gateway introductory decks and CR 26.03 §1.6 setup.
use super::*;

// Official System Gateway product list, also recorded in foundation 02.
const DECKS: &[(CardKind, usize)] = &[
    (CardKind::TreadLightly, 2),
    (CardKind::CreativeCommission, 2),
    (CardKind::VRcation, 2),
    (CardKind::Overclock, 2),
    (CardKind::Jailbreak, 3),
    (CardKind::SureGamble, 3),
    (CardKind::DocklandsPass, 1),
    (CardKind::Pennyshaver, 1),
    (CardKind::Cleaver, 2),
    (CardKind::Carmen, 2),
    (CardKind::Unity, 2),
    (CardKind::Mayfly, 2),
    (CardKind::RedTeam, 1),
    (CardKind::TeleworkContract, 2),
    (CardKind::SmartwareDistributor, 2),
    (CardKind::VerbalPlasticity, 1),
    (CardKind::OffworldOffice, 3),
    (CardKind::SendAMessage, 2),
    (CardKind::SuperconductingHub, 2),
    (CardKind::NicoCampaign, 2),
    (CardKind::UrticaCipher, 2),
    (CardKind::RegolithMiningLicense, 2),
    (CardKind::Bran, 2),
    (CardKind::Diviner, 2),
    (CardKind::Karuna, 2),
    (CardKind::Palisade, 3),
    (CardKind::Tithe, 2),
    (CardKind::Whitespace, 2),
    (CardKind::SeamlessLaunch, 2),
    (CardKind::GovernmentSubsidy, 2),
    (CardKind::HedgeFund, 3),
    (CardKind::ManegarmSkunkworks, 1),
];

impl Match {
    /// Seeded construction is for reproducible engineering checks. Native play
    /// supplies fresh entropy, independently of any opponent policy.
    pub fn introductory(seed: u64) -> Self {
        let cards = DECKS
            .iter()
            .flat_map(|&(kind, count)| std::iter::repeat_n(kind, count))
            .enumerate()
            .map(|(index, kind)| {
                CardInstance::new(
                    CardId(index as u32 + 1),
                    kind,
                    if kind.owner() == Player::Corp {
                        Zone::Rd
                    } else {
                        Zone::Stack
                    },
                    false,
                )
            })
            .collect();
        let mut state = State {
            information_revision: 0,
            corp_credits: 5,
            revision: 0,
            cards,
            hq_positions: vec![],
            run: None,
            run_result: None,
            pending: Some(Pending::Mulligan {
                actor: Player::Corp,
            }),
            history: vec![],
            turn: Some(Turn {
                introductory: true,
                active: Player::Corp,
                number: 0,
                clicks: [0, 0],
                runner_credits: 5,
                points: [0, 0],
                winner: None,
                terminal: None,
                selected: vec![],
                random_state: seed,
                random_algorithm: "unfolded-tabletop-splitmix64-v1".into(),
                next_remote: 1,
                run: None,
                hq_breached: false,
                successful_run: false,
                central_runs: vec![],
                runner_basic_draw_taken: false,
                income_pending: vec![],
            }),
        };
        state.log("System Gateway introductory game: The Syndicate versus The Catalyst; 6 agenda points to win. Each player takes 5 credits and draws 5 cards.".into());
        for player in [Player::Corp, Player::Runner] {
            state.shuffle_starting_hand(player);
        }
        Self(state, undo::Correction::default())
    }

    pub(crate) fn is_introductory(&self) -> bool {
        self.0.turn.as_ref().is_some_and(|turn| turn.introductory)
    }
    pub(crate) fn start_after(&mut self, revision: u64) -> Result<(), &'static str> {
        self.0.revision = revision
            .checked_add(1)
            .ok_or("Decision revision exhausted")?;
        self.0.information_revision = self.0.revision;
        if !self.0.valid() {
            return Err("Decision revision exhausted");
        }
        Ok(())
    }
}

impl State {
    pub(super) fn exact_introductory_decks(&self) -> bool {
        self.cards.len() == 64
            && DECKS.iter().all(|&(kind, count)| {
                self.cards.iter().filter(|card| card.kind == kind).count() == count
            })
    }

    fn shuffle_starting_hand(&mut self, player: Player) {
        let (deck, hand) = if player == Player::Corp {
            (Zone::Rd, Zone::Hq)
        } else {
            (Zone::Stack, Zone::Grip)
        };
        let mut random = SeededRandom::new(self.turn().random_state);
        let indices: Vec<_> = self
            .cards
            .iter()
            .enumerate()
            .filter_map(|(index, card)| (card.zone == deck || card.zone == hand).then_some(index))
            .collect();
        let mut shuffled: Vec<_> = indices
            .iter()
            .map(|&index| self.cards[index].clone())
            .collect();
        random.shuffle(&mut shuffled);
        for (position, (index, mut card)) in indices.into_iter().zip(shuffled).enumerate() {
            card.zone = if position < 5 { hand } else { deck };
            self.cards[index] = card;
        }
        self.turn_mut().random_state = random.state();
        self.information_revision = self.revision;
    }

    pub(super) fn finish_mulligan(&mut self, actor: Player, mulligan: bool) {
        if mulligan {
            self.shuffle_starting_hand(actor);
            self.log(format!("{actor:?} shuffles the starting hand back into the deck and draws 5 cards; the new hand must be kept."));
        } else {
            self.log(format!("{actor:?} keeps the starting hand."));
        }
        if actor == Player::Corp {
            self.pending = Some(Pending::Mulligan {
                actor: Player::Runner,
            });
        } else {
            self.turn_mut().number = 1;
            self.turn_mut().clicks = [3, 0];
            self.paid(TurnWindow::Start);
            self.log("Corp begins turn 1 with 3 clicks.".into());
        }
    }
}
