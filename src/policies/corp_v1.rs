//! Introductory Corp priorities, authored from the fixed admitted mechanics.
//! Card identifiers are exact supported printings, never parsed rules prose.
use crate::game::{Action, CardId, CardView, Observation, Server, ServerView};

fn agenda(card: &CardView) -> Option<u32> {
    match card.printing {
        Some("30067") => Some(4),
        Some("30069") => Some(5),
        Some("30070") => Some(3),
        _ => None,
    }
}
fn card(view: &Observation, id: CardId) -> Option<&CardView> {
    view.hand
        .iter()
        .chain(&view.rig)
        .chain(
            view.servers
                .iter()
                .flat_map(|s| s.root.iter().chain(&s.ice)),
        )
        .find(|c| c.instance == Some(id))
}

pub(super) fn response(view: &Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if choices.contains(&Action::KeepHand) {
        let defense = view
            .hand
            .iter()
            .any(|c| ice_cost(c).is_some_and(|cost| cost <= view.corp_credits));
        let economy = view
            .hand
            .iter()
            .any(|c| matches!(c.printing, Some("30075" | "30037" | "30071")));
        return Some(if defense || economy {
            Action::KeepHand
        } else {
            Action::Mulligan
        });
    }
    if choices.contains(&Action::ConfirmDiscard) {
        return Some(Action::ConfirmDiscard);
    }
    // Retain existing defense unless completing this pending installation
    // actually requires trashing a displaced card or reducing its install cost.
    if choices.contains(&Action::FinishInstall) {
        return Some(Action::FinishInstall);
    }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::TrashInstalled(id) => {
                card(view, *id).map(|c| (*a, keep_value(c) + c.advancement * 10))
            }
            _ => None,
        })
        .min_by_key(|(_, value)| *value)
    {
        return Some(action.0);
    }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::ToggleDiscard(id) => card(view, *id)
                .filter(|c| !c.selected)
                .map(|c| (*a, keep_value(c))),
            _ => None,
        })
        .min_by_key(|(_, value)| *value)
    {
        return Some(action.0);
    }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::ResolveIncome(_)))
    {
        return Some(*action);
    }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::RezIceFree(id) => card(view, *id).and_then(ice_cost).map(|cost| (*a, cost)),
            _ => None,
        })
        .max_by_key(|(_, cost)| *cost)
    {
        return Some(action.0);
    }
    if view.run.is_some() && let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(id) if card(view,*id).is_some_and(|c| ice_cost(c).is_some()))) { return Some(*action); }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::InstallInward(id) => card(view, *id)
                .and_then(ice_cost)
                .map(|cost| {
                    cost + if view.run.as_ref().is_some_and(|run| run.tread_lightly) {
                        3
                    } else {
                        0
                    }
                })
                .filter(|cost| *cost <= view.corp_credits)
                .map(|cost| (*a, cost)),
            _ => None,
        })
        .min_by_key(|(_, cost)| *cost)
    {
        return Some(action.0);
    }
    if choices.contains(&Action::DeclineInstall) {
        return Some(Action::DeclineInstall);
    }
    if choices.contains(&Action::DrawAgendaCards) {
        return Some(if view.rd_count > 2 && view.hand.len() < 5 {
            Action::DrawAgendaCards
        } else {
            Action::DeclineAgendaAbility
        });
    }
    if choices.contains(&Action::DeclineAgendaAbility) {
        return Some(Action::DeclineAgendaAbility);
    }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::SelectTarget(id) => card(view, *id).map(|c| {
                (
                    *a,
                    agenda(c).map_or(0, |n| 100 + 10 * n.saturating_sub(c.advancement).min(2)),
                )
            }),
            _ => None,
        })
        .max_by_key(|(_, value)| *value)
    {
        return Some(action.0);
    }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::Score(id) => card(view, *id)
                .and_then(agenda)
                .map(|requirement| (*a, requirement)),
            _ => None,
        })
        .max_by_key(|(_, requirement)| *requirement)
    {
        return Some(action.0);
    }
    if view.servers.iter().flat_map(|s|&s.root).any(|c|!c.installed_this_turn && agenda(c).is_some_and(|n|n.saturating_sub(c.advancement)>=2))
        && let Some(action)=choices.iter().find(|a|matches!(a,Action::PlayCard(id) if card(view,*id).is_some_and(|c|c.printing==Some("30040")))) { return Some(*action); }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(id) if card(view,*id).is_some_and(|c| match c.printing {
        Some("30037"|"30071") => view.corp_credits >= 2 + rez_reserve(view),
        Some("30042") => view.run.as_ref().is_some_and(|run|view.servers.iter().any(|s|s.server==run.server && s.root.iter().any(|c|c.instance==Some(*id)))),
        // Urtica works while unrezzed; paying to expose it buys nothing.
        _=>false,
    }))) { return Some(*action); }
    if let Some(card) = view.servers.iter().flat_map(|s| &s.root).find(|c| {
        let needed = match c.printing {
            Some("30067") => 4u32,
            Some("30069") => 5,
            Some("30070") => 3,
            _ => return false,
        }
        .saturating_sub(c.advancement);
        needed <= u32::from(view.clicks[0]) && needed <= view.corp_credits
    }) && let Some(id) = card.instance
        && choices.contains(&Action::Advance(id))
    {
        return Some(Action::Advance(id));
    }
    // Use the scoring server already built, rather than installing ICE in an
    // endless series of new remotes while agendas remain in HQ.
    if view.servers.iter().any(|s| {
        matches!(s.server, Server::Remote(_))
            && !s.ice.is_empty()
            && s.root.iter().all(|c| c.printing == Some("30042"))
    }) && !view
        .servers
        .iter()
        .flat_map(|s| &s.root)
        .any(|c| agenda(c).is_some())
        && let Some(action) = choices
            .iter()
            .filter_map(|a| match a {
                Action::InstallCard(id) => card(view, *id).and_then(agenda).map(|n| (*a, n)),
                _ => None,
            })
            .min_by_key(|(_, n)| *n)
    {
        return Some(action.0);
    }
    let defense = defense_destination(view);
    if view
        .installing
        .as_ref()
        .is_some_and(|c| ice_cost(c).is_some())
        && choices.contains(&Action::Destination(defense))
    {
        return Some(Action::Destination(defense));
    }
    let layers = view
        .servers
        .iter()
        .find(|s| s.server == defense)
        .map_or(0, |s| s.ice.len() as u32);
    if layers < 2
        && let Some(action) = choices
            .iter()
            .filter_map(|a| match a {
                Action::InstallCard(id) => card(view, *id)
                    .and_then(ice_cost)
                    .filter(|cost| cost + layers + rez_reserve(view) <= view.corp_credits)
                    .map(|cost| (*a, cost)),
                _ => None,
            })
            .min_by_key(|(_, cost)| *cost)
    {
        return Some(action.0);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::PlayCard(id) if card(view,*id).is_some_and(|c| matches!(c.printing,Some("30075"|"30064"))))) {
        return Some(*action);
    }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::TakeRegolith(_)))
    {
        return Some(*action);
    }
    let upgrade_server = view.servers.iter().find(|s| {
        s.root.iter().any(|c| agenda(c).is_some())
            && !s.root.iter().any(|c| c.printing == Some("30042"))
    });
    if let Some(server) = upgrade_server {
        if view
            .installing
            .as_ref()
            .is_some_and(|c| c.printing == Some("30042"))
            && choices.contains(&Action::Destination(server.server))
        {
            return Some(Action::Destination(server.server));
        }
        if view.corp_credits >= 2 + rez_reserve(view) && let Some(action)=choices.iter().find(|a|matches!(a,Action::InstallCard(id) if card(view,*id).is_some_and(|c|c.printing==Some("30042")))) { return Some(*action); }
    }
    if let Some(action) = choices.iter().find(
        |a| matches!(a, Action::Advance(id) if card(view,*id).is_some_and(|c| agenda(c).is_some())),
    ) {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::InstallCard(id) if card(view,*id).is_some_and(|c| agenda(c).is_some()))) {
        return Some(*action);
    }
    if view.corp_credits >= 2 + rez_reserve(view) && view.servers.iter().flat_map(|s|&s.root).filter(|c|matches!(c.printing,Some("30037"|"30071"))).count()<2
        && let Some(action) = choices.iter().find(|a|matches!(a,Action::InstallCard(id) if card(view,*id).is_some_and(|c|matches!(c.printing,Some("30037"|"30071"))))) { return Some(*action); }
    if view.installing.is_some()
        && let Some(destination) = choices
            .iter()
            .filter_map(|a| match a {
                Action::Destination(server) => Some((
                    *a,
                    view.servers
                        .iter()
                        .find(|s| s.server == *server)
                        .filter(|s| s.root.iter().all(|c| c.printing == Some("30042")))
                        .map_or(
                            if *server == Server::NewRemote {
                                1
                            } else {
                                -100
                            },
                            |s| 2 + s.ice.len() as i32 * 10,
                        ),
                )),
                _ => None,
            })
            .max_by_key(|(_, value)| *value)
    {
        return Some(destination.0);
    }
    if view.hand.len() < 3
        && !view.hand.iter().any(|c| agenda(c).is_some())
        && view.rd_count > 2
        && view.corp_credits >= 5 + rez_reserve(view)
        && choices.contains(&Action::Draw)
    {
        return Some(Action::Draw);
    }
    // There are no virus counters in the admitted introductory rig, so Purge
    // buys no effect. Unneeded traps/upgrades and further ICE lose to economy.
    [Action::PassPriority, Action::GainCredit]
        .into_iter()
        .find(|action| choices.contains(action))
}

fn ice_cost(card: &CardView) -> Option<u32> {
    match card.printing {
        Some("30039") => Some(6),
        Some("30072") => Some(3),
        Some("30074") => Some(2),
        Some("30046") => Some(2),
        Some("30047") => Some(4),
        Some("30073") => Some(1),
        _ => None,
    }
}

fn keep_value(card: &CardView) -> u32 {
    if let Some(n) = agenda(card) {
        60 - n
    } else if let Some(cost) = ice_cost(card) {
        40 - cost
    } else {
        match card.printing {
            Some("30075" | "30064") => 70,
            Some("30037" | "30071") => 45,
            Some("30040") => 50,
            Some("30042") => 25,
            Some("30045") => 10,
            _ => 0,
        }
    }
}

fn server_value(view: &Observation, server: &ServerView) -> u32 {
    let agenda_count =
        |cards: &[CardView]| cards.iter().filter(|c| agenda(c).is_some()).count() as u32;
    match server.server {
        Server::Hq => 70 + 20 * agenda_count(&view.hand),
        Server::Rd => {
            if view.rd_count > 0 {
                65
            } else {
                0
            }
        }
        Server::Archives => 50 * agenda_count(&view.archives),
        Server::Remote(_) => {
            if agenda_count(&server.root) > 0 {
                120
            } else if server.root.is_empty() {
                45
            } else {
                25
            }
        }
        _ => 0,
    }
}
fn defense_destination(view: &Observation) -> Server {
    let best = view
        .servers
        .iter()
        .max_by_key(|s| server_value(view, s) / (1 + 3 * s.ice.len() as u32));
    if view.hand.iter().any(|c| agenda(c).is_some())
        && best.is_none_or(|s| server_value(view, s) / (1 + 3 * s.ice.len() as u32) < 45)
    {
        Server::NewRemote
    } else {
        best.map_or(Server::Rd, |s| s.server)
    }
}
fn rez_reserve(view: &Observation) -> u32 {
    view.servers
        .iter()
        .filter(|s| server_value(view, s) >= 45)
        .flat_map(|s| &s.ice)
        .filter(|c| !c.rezzed)
        .filter_map(ice_cost)
        .max()
        .unwrap_or(0)
}
