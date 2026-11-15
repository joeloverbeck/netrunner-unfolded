//! Frozen incumbent from db313eed. New policies must leave this source unchanged.
use crate::game::{Action, Observation, Player, Server};
pub(super) fn response(observation: &Observation) -> Option<Action> {
    if observation.phase.starts_with("Setup") {
        return observation.decision.as_ref().and_then(|decision| {
            decision
                .choices
                .contains(&Action::KeepHand)
                .then_some(Action::KeepHand)
        });
    }
    if observation.active_player.is_none() {
        return crate::game::fixed_runner_response(observation);
    }
    baseline(observation)
}

pub(super) fn baseline(view: &Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if view.phase.starts_with("Karunā") && choices.contains(&Action::JackOut) {
        return Some(
            if view.hand.len() < 2
                && view
                    .run
                    .as_ref()
                    .is_some_and(|r| r.subroutines.get(1).is_some_and(|s| !s.broken))
            {
                Action::JackOut
            } else {
                Action::Continue
            },
        );
    }
    if choices.contains(&Action::DrawAgendaCards) {
        return Some(if view.rd_count < 2 {
            Action::DeclineAgendaAbility
        } else {
            Action::DrawAgendaCards
        });
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::RezIceFree(_))) {
        return Some(*action);
    }
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
        if let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(id) if !view.servers.iter().flat_map(|s| &s.root).any(|c| c.instance == Some(*id) && c.printing == Some("30045")))) {
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
            && run.subroutines.iter().any(|s| !s.broken)
            && let Some((id, _, _)) = run.breakers.iter().find(|(id, title, strength)| {
                *strength < run.ice_strength.unwrap_or(0)
                    && choices.contains(&Action::Boost(*id))
                    && ((title == "Cleaver" && matches!(ice.printing, Some("30039" | "30072")))
                        || (title == "Unity" && matches!(ice.printing, Some("30074" | "30046")))
                        || (title == "Carmen" && matches!(ice.printing, Some("30073" | "30047")))
                        || title == "Mayfly")
            })
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
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::PlayCard(id) if card(*id).is_some_and(|c| matches!(c.printing, Some("30030" | "30075" | "30064" | "30020"))))) { return Some(*action); }
    // Invest in repeatable drawing while the public stack count can supply it;
    // do not replace an active unique copy merely because installation is legal.
    if view.stack_count >= 2 && !view.rig.iter().any(|c| c.printing == Some("30034"))
        && let Some(action) = choices.iter().find(|a| matches!(a, Action::InstallCard(id) if card(*id).is_some_and(|c| c.printing == Some("30034")))) {
        return Some(*action);
    }
    if view.hand.len() <= 2 && view.stack_count > 0
        && let Some(action) = choices.iter().find(|a| matches!(a, Action::PlayCard(id) if card(*id).is_some_and(|c| c.printing == Some("30021")))) {
        return Some(*action);
    }
    if view.active_player == Some(Player::Runner)
        && view.hand.len() <= 3
        && view.stack_count > 0
        && !view.runner_basic_draw_taken
        && view.rig.iter().any(|c| c.printing == Some("30034"))
        && choices.contains(&Action::Draw)
    {
        return Some(Action::Draw);
    }
    if let Some(action) = choices.iter().find(|a| {
        matches!(
            a,
            Action::ResolveIncome(_) | Action::TakeTelework(_) | Action::TakeRegolith(_)
        )
    }) {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::Rez(id) if card(*id).is_some_and(|c| matches!(c.printing, Some("30037" | "30071"))))) { return Some(*action); }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::LoadSmartware(id) if view.runner_credits >= 3 && card(*id).is_some_and(|c| c.hosted_credits < 3))) { return Some(*action); }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::InstallCard(id) if card(*id).is_some_and(|c| matches!(c.printing, Some("30027" | "30033" | "30037" | "30071"))))) { return Some(*action); }
    // Collect a useful reserve, or recover a low pool. Hosted savings are not
    // available to the payment planner before this click action.
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::TakePennyshaver(id) if card(*id).is_some_and(|c| c.hosted_credits >= 2 || (c.hosted_credits > 0 && view.runner_credits < 3)))) {
        return Some(*action);
    }
    // Count only public defenses; an unrezzed ICE's identity is unavailable.
    let defenses = |server: Server| {
        view.servers
            .iter()
            .find(|s| s.server == server)
            .map_or(0, |s| s.ice.len())
    };
    if let Some(action) = choices
        .iter()
        .filter(|a| matches!(a, Action::RunRedTeam { .. }))
        .min_by_key(|a| {
            if let Action::RunRedTeam { server, .. } = a {
                defenses(*server)
            } else {
                usize::MAX
            }
        })
    {
        return Some(*action);
    }
    if let Some(action) = choices
        .iter()
        .filter(|a| matches!(a, Action::RunServer(_)))
        .min_by_key(|a| {
            if let Action::RunServer(server) = a {
                defenses(*server)
            } else {
                usize::MAX
            }
        })
        && view.phase.starts_with("Tread Lightly")
    {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::InstallCard(id) if card(*id).is_some_and(|c| c.printing == Some("30018") || (c.printing == Some("30014") && !view.rig.iter().any(|c| c.printing == Some("30014")))))) {
        return Some(*action);
    }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::Advance(id) if card(*id).is_some_and(|c| c.advancement < match c.printing { Some("30069") => 5, Some("30070") => 3, _ => 4 })))
    {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::InstallCard(id) if card(*id).is_some_and(|c| matches!(c.printing, Some("30067" | "30069" | "30070" | "30006" | "30026" | "30013"))))) { return Some(*action); }
    if let Some(action) = choices.iter().find(
        |a| matches!(a,Action::PlayCard(id) if card(*id).is_some_and(|c|c.printing==Some("30028"))),
    ) {
        return Some(*action);
    }
    if choices.contains(&Action::GainCredit) {
        if let Some(action) = choices.iter().find(|a| matches!(a, Action::PlayCard(id) if card(*id).is_some_and(|c| c.printing == Some("30012")))) {
            return Some(*action);
        }
        return Some(Action::GainCredit);
    }
    if choices.contains(&Action::PassPriority) {
        return Some(Action::PassPriority);
    }
    choices.first().copied()
}
