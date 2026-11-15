//! Finite public mechanics for the admitted breakers/ICE. These same quantities
//! drive engine legality and local cost plans; no hidden state or transitions.
use super::{CardKind, CardView, Server};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IceFamily {
    Barrier,
    CodeGate,
    Sentry,
}
#[derive(Clone, Copy, Debug)]
pub struct BreakerMechanics {
    pub family: Option<IceFamily>,
    pub strength: u64,
    pub boost_cost: u8,
    pub boost_amount: u64,
    pub break_capacity: u8,
    pub disposable: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IceEffect {
    Bran,
    Palisade,
    Whitespace,
    Tithe,
    Diviner,
    Karuna,
}
#[derive(Clone, Copy, Debug)]
pub struct IceMechanics {
    pub family: IceFamily,
    pub strength: u64,
    pub count: u8,
    pub effect: IceEffect,
}
impl CardKind {
    pub(super) fn breaker_mechanics(self, installed: usize) -> Option<BreakerMechanics> {
        let (family, strength, boost_cost, boost_amount, break_capacity, disposable) = match self {
            Self::Cleaver => (Some(IceFamily::Barrier), 3, 2, 1, 2, false),
            Self::Unity => (Some(IceFamily::CodeGate), 1, 1, installed as u64, 1, false),
            Self::Carmen => (Some(IceFamily::Sentry), 2, 2, 3, 1, false),
            Self::Mayfly => (None, 1, 1, 1, 1, true),
            _ => return None,
        };
        Some(BreakerMechanics {
            family,
            strength,
            boost_cost,
            boost_amount,
            break_capacity,
            disposable,
        })
    }
    pub(super) fn ice_mechanics(self, server: Server) -> Option<IceMechanics> {
        let (family, strength, count, effect) = match self {
            Self::Bran => (IceFamily::Barrier, 6, 3, IceEffect::Bran),
            Self::Palisade => (
                IceFamily::Barrier,
                if matches!(server, Server::Remote(_)) {
                    4
                } else {
                    2
                },
                1,
                IceEffect::Palisade,
            ),
            Self::Whitespace => (IceFamily::CodeGate, 0, 2, IceEffect::Whitespace),
            Self::Tithe => (IceFamily::Sentry, 1, 2, IceEffect::Tithe),
            Self::Diviner => (IceFamily::CodeGate, 3, 1, IceEffect::Diviner),
            Self::Karuna => (IceFamily::Sentry, 3, 2, IceEffect::Karuna),
            _ => return None,
        };
        Some(IceMechanics {
            family,
            strength,
            count,
            effect,
        })
    }
}
impl CardView {
    fn tactical_kind(&self) -> Option<CardKind> {
        Some(match self.printing? {
            "30006" => CardKind::Cleaver,
            "30026" => CardKind::Unity,
            "30015" => CardKind::Carmen,
            "30032" => CardKind::Mayfly,
            "30039" => CardKind::Bran,
            "30072" => CardKind::Palisade,
            "30074" => CardKind::Whitespace,
            "30073" => CardKind::Tithe,
            "30046" => CardKind::Diviner,
            "30047" => CardKind::Karuna,
            _ => return None,
        })
    }
    pub fn breaker_mechanics(&self, installed: usize) -> Option<BreakerMechanics> {
        self.tactical_kind()?.breaker_mechanics(installed)
    }
    pub fn ice_mechanics(&self, server: Server) -> Option<IceMechanics> {
        self.tactical_kind()?.ice_mechanics(server)
    }
}
