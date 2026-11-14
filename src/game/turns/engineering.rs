//! Exact #7 hostile input; metadata is not admission of unimplemented effects.
use super::*;
impl EngineeringPosition {
    /// #16's mixed damage/access case and its Karunā continuation. Exact
    /// introductory inventory; no ordinary-play preset or second rules path.
    pub fn damage_run(seed: u64, karuna: bool) -> Self {
        use CardKind::*;
        let mut position = Self::hq_access(seed);
        position.corp_credits = 0;
        position.runner_credits = 8;
        for card in &mut position.cards {
            card.zone = if card.kind.owner() == Player::Corp {
                Zone::Rd
            } else {
                Zone::Stack
            };
            card.rezzed = false;
            card.advancement = 0;
            card.installed_turn = None;
        }
        let mut remaining = std::mem::take(&mut position.cards);
        let mut selected = if karuna {
            vec![(SureGamble, Zone::Grip); 3]
        } else {
            vec![
                (Overclock, Zone::Grip),
                (DocklandsPass, Zone::Grip),
                (Jailbreak, Zone::Grip),
            ]
        };
        selected.extend([(Jailbreak, Zone::Grip); 2]);
        selected.push((UrticaCipher, Zone::Root(Server::Remote(1))));
        if karuna {
            selected.push((Karuna, Zone::Protecting(Server::Remote(1))));
        } else {
            // Vector order is innermost first.
            selected.extend([
                (Diviner, Zone::Protecting(Server::Remote(1))),
                (Tithe, Zone::Protecting(Server::Remote(1))),
            ]);
        }
        selected.extend([
            (HedgeFund, Zone::Rd),
            (HedgeFund, Zone::Rd),
            (SeamlessLaunch, Zone::Rd),
        ]);
        for (kind, zone) in selected {
            let index = remaining.iter().position(|c| c.kind == kind).unwrap();
            let mut card = remaining.remove(index);
            card.zone = zone;
            if matches!(zone, Zone::Protecting(_) | Zone::Root(_)) {
                card.installed_turn = Some(1);
                card.rezzed = matches!(zone, Zone::Protecting(_));
            }
            position.cards.push(card);
        }
        position.cards.extend(remaining);
        position
    }
    /// All 30 Runner / 34 Corp nonidentity cards, with exact introductory
    /// multiplicities. Unsupported remainder stays inactive in its owner's
    /// deck. A transition that makes it live is refused without mutation.
    pub fn hq_access(seed: u64) -> Self {
        use CardKind::*;
        let mut position = Self::economy(seed);
        position.active = Player::Runner;
        position.corp_credits = 3;
        position.runner_credits = 8;
        position.shuffle_decks = false;
        position.cards = vec![
            EngineeringCard::new(Overclock, Zone::Grip),
            EngineeringCard::new(Palisade, Zone::Hq),
            EngineeringCard::new(OffworldOffice, Zone::Hq),
            EngineeringCard::new(HedgeFund, Zone::Hq),
            EngineeringCard::new(SeamlessLaunch, Zone::Rd),
        ];
        for (kind, zone, rezzed, advancement) in [
            (Whitespace, Zone::Protecting(Server::Hq), true, 0),
            (Bran, Zone::Protecting(Server::Hq), true, 0),
            (Cleaver, Zone::Rig, false, 0),
            (Unity, Zone::Rig, false, 0),
            (DocklandsPass, Zone::Rig, false, 0),
            (ManegarmSkunkworks, Zone::Root(Server::Hq), true, 0),
            (OffworldOffice, Zone::Root(Server::Remote(1)), false, 2),
        ] {
            position.cards.push(EngineeringCard {
                kind,
                zone,
                rezzed,
                advancement,
                installed_turn: Some(1),
            });
        }
        for (kind, copies) in [
            (TreadLightly, 2),
            (CreativeCommission, 2),
            (VRcation, 2),
            (Overclock, 2),
            (Jailbreak, 3),
            (SureGamble, 3),
            (DocklandsPass, 1),
            (Pennyshaver, 1),
            (Cleaver, 2),
            (Carmen, 2),
            (Unity, 2),
            (Mayfly, 2),
            (RedTeam, 1),
            (TeleworkContract, 2),
            (SmartwareDistributor, 2),
            (VerbalPlasticity, 1),
            (OffworldOffice, 3),
            (SendAMessage, 2),
            (SuperconductingHub, 2),
            (NicoCampaign, 2),
            (UrticaCipher, 2),
            (RegolithMiningLicense, 2),
            (Bran, 2),
            (Diviner, 2),
            (Karuna, 2),
            (Palisade, 3),
            (Tithe, 2),
            (Whitespace, 2),
            (SeamlessLaunch, 2),
            (GovernmentSubsidy, 2),
            (HedgeFund, 3),
            (ManegarmSkunkworks, 1),
        ] {
            let used = position.cards.iter().filter(|c| c.kind == kind).count();
            let deck = if kind.owner() == Player::Runner {
                Zone::Stack
            } else {
                Zone::Rd
            };
            position
                .cards
                .extend((used..copies).map(|_| EngineeringCard::new(kind, deck)));
        }
        position
    }
}

impl EngineeringPosition {
    /// #7's independently selected winning-steal case and its below-threshold
    /// control, retaining the exact introductory deck multiplicities.
    pub fn central_access(seed: u64, winning: bool) -> Self {
        let mut position = Self::hq_access(seed);
        position.corp_credits = 0;
        position.runner_credits = 5;
        position.agenda_points = [0, if winning { 3 } else { 2 }];
        for card in &mut position.cards {
            card.zone = if card.kind.owner() == Player::Corp {
                Zone::Rd
            } else {
                Zone::Stack
            };
            card.rezzed = false;
            card.advancement = 0;
            card.installed_turn = None;
        }
        let mut remaining = std::mem::take(&mut position.cards);
        for (kind, zone) in [
            (CardKind::Jailbreak, Zone::Grip),
            (CardKind::SureGamble, Zone::Stack),
            (
                if winning {
                    CardKind::SendAMessage
                } else {
                    CardKind::OffworldOffice
                },
                Zone::RunnerScore,
            ),
            (CardKind::SendAMessage, Zone::Rd),
            (CardKind::HedgeFund, Zone::Rd),
            (CardKind::Palisade, Zone::Protecting(Server::Hq)),
        ] {
            let index = remaining.iter().position(|c| c.kind == kind).unwrap();
            let mut card = remaining.remove(index);
            card.zone = zone;
            if matches!(zone, Zone::Protecting(_)) {
                card.installed_turn = Some(1);
            }
            position.cards.push(card);
        }
        position.cards.extend(remaining);
        position
    }
}
