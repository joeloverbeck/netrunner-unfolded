//! Lawful, deterministic introductory Runner. Only current player observations
//! enter this module. Cost lines cover visible mechanics, not hypothetical worlds.
use crate::game::{Action, CardId, CardView, IceEffect, IceMechanics, Observation, Server};

fn card(view: &Observation, id: CardId) -> Option<&CardView> {
    view.hand
        .iter()
        .chain(&view.rig)
        .find(|c| c.instance == Some(id))
}
#[derive(Clone, Copy)]
struct Line {
    pool: u32,
    hosted: u32,
    clicks: u8,
    hand: usize,
    lost: u32,
    cost: u32,
    first: Option<Action>,
}
impl Line {
    fn pay(mut self, n: u32) -> Option<Self> {
        let temporary = n.min(self.hosted);
        self.hosted -= temporary;
        let permanent = n - temporary;
        if permanent > self.pool {
            return None;
        }
        self.pool -= permanent;
        self.cost += permanent;
        Some(self)
    }
    fn start(view: &Observation) -> Self {
        Self {
            pool: view.runner_credits,
            hosted: view.run.as_ref().map_or(0, |r| u32::from(r.hosted_credits)),
            clicks: view.clicks[1],
            hand: view.hand.len(),
            lost: 0,
            cost: 0,
            first: None,
        }
    }
}
// At most three subroutines and four admitted breaker types. Enumerate the
// relevant masks and click alternatives; do not search a cloned game state.
fn ice_lines(
    view: &Observation,
    ice: IceMechanics,
    broken: u8,
    current: bool,
    line: Line,
) -> Vec<Line> {
    let all = (1u8 << ice.count) - 1;
    let remaining = all & !broken;
    let installed = view
        .rig
        .iter()
        .filter(|c| c.breaker_mechanics(1).is_some())
        .count();
    let mut out = vec![];
    for mask in 0..=all {
        if mask & !remaining != 0 {
            continue;
        }
        for click_mask in 0..=all {
            if click_mask & !mask != 0 || (click_mask != 0 && ice.effect != IceEffect::Bran) {
                continue;
            }
            let clicks = click_mask.count_ones() as u8;
            if clicks > line.clicks {
                continue;
            }
            let paid_mask = mask & !click_mask;
            // The no-breaker case permits deliberate damage or click-only lines.
            for index in 0..=view.rig.len() {
                let mut next = line;
                next.clicks -= clicks;
                next.cost += u32::from(clicks) * 2;
                let mut first = (click_mask != 0)
                    .then(|| Action::BreakWithClick(click_mask.trailing_zeros() as u8 + 1));
                if paid_mask != 0 {
                    let Some(breaker) = view.rig.get(index) else {
                        continue;
                    };
                    let Some(rules) = breaker.breaker_mechanics(installed) else {
                        continue;
                    };
                    if rules.family.is_some_and(|family| family != ice.family) {
                        continue;
                    }
                    let Some(id) = breaker.instance else {
                        continue;
                    };
                    let strength = if current {
                        view.run
                            .as_ref()
                            .and_then(|r| r.breakers.iter().find(|b| b.0 == id))
                            .map_or(rules.strength, |b| b.2)
                    } else {
                        rules.strength
                    };
                    let boost = ice
                        .strength
                        .saturating_sub(strength)
                        .div_ceil(rules.boost_amount.max(1));
                    let cost = boost as u32 * u32::from(rules.boost_cost)
                        + paid_mask
                            .count_ones()
                            .div_ceil(u32::from(rules.break_capacity));
                    let Some(paid) = next.pay(cost) else {
                        continue;
                    };
                    next = paid;
                    if rules.disposable {
                        let bit = 1u32 << index;
                        if next.lost & bit == 0
                            && !view
                                .run
                                .as_ref()
                                .is_some_and(|r| r.trash_at_run_end.contains(&id))
                        {
                            next.cost += 4;
                        }
                        next.lost |= bit;
                    }
                    // Only use a legal prefix now; the authoritative engine
                    // supplies fresh choices after each boost/break/payment.
                    let mut first_mask = 0u8;
                    for bit in 0..ice.count {
                        if paid_mask & (1 << bit) != 0
                            && first_mask.count_ones() < u32::from(rules.break_capacity)
                        {
                            first_mask |= 1u8 << bit;
                        }
                    }
                    first = Some(if boost > 0 {
                        Action::Boost(id)
                    } else {
                        Action::Break {
                            breaker: id,
                            subroutines: first_mask,
                        }
                    });
                } else if index != view.rig.len() {
                    continue;
                }
                let unbroken = remaining & !mask;
                let damage = match ice.effect {
                    IceEffect::Karuna => 2 * (unbroken & 3).count_ones() as usize,
                    IceEffect::Tithe => usize::from(unbroken & 1 != 0),
                    _ => 0,
                };
                if damage > next.hand {
                    continue;
                }
                next.hand -= damage;
                next.cost += damage as u32 * 2;
                match ice.effect {
                    IceEffect::Bran if unbroken & 6 != 0 => continue,
                    IceEffect::Bran if unbroken & 1 != 0 => {
                        next.cost += 4;
                    } // unknown inward installation remains risk
                    IceEffect::Palisade if unbroken != 0 => continue,
                    IceEffect::Diviner if unbroken != 0 => continue, // random discarded card may end the run
                    IceEffect::Whitespace => {
                        if unbroken & 1 != 0 {
                            let loss = next.pool.min(3);
                            next.pool -= loss;
                            next.cost += loss;
                        }
                        if unbroken & 2 != 0 && next.pool <= 6 {
                            continue;
                        }
                    }
                    IceEffect::Tithe if unbroken & 2 != 0 => next.cost += 1,
                    _ => {}
                }
                if next.first.is_none() {
                    next.first = first;
                }
                out.push(next);
            }
        }
    }
    out
}
fn prune(lines: Vec<Line>) -> Vec<Line> {
    let mut kept: Vec<Line> = vec![];
    for line in lines {
        let dominates = |a: &Line, b: &Line| {
            a.pool >= b.pool
                && a.hosted >= b.hosted
                && a.clicks >= b.clicks
                && a.hand >= b.hand
                && a.cost <= b.cost
                && a.lost & !b.lost == 0
        };
        if kept.iter().any(|a| dominates(a, &line)) {
            continue;
        }
        kept.retain(|b| !dominates(&line, b));
        kept.push(line);
    }
    kept
}
fn rest(view: &Observation, server: Server, inner: usize, mut lines: Vec<Line>) -> Vec<Line> {
    let Some(board) = view.servers.iter().find(|s| s.server == server) else {
        return lines;
    };
    for c in board.ice.iter().take(inner).rev() {
        if let Some(ice) = c.ice_mechanics(server).filter(|_| c.rezzed) {
            lines = prune(
                lines
                    .into_iter()
                    .flat_map(|line| ice_lines(view, ice, 0, false, line))
                    .collect(),
            );
        } else {
            for line in &mut lines {
                line.cost += 3;
            }
        } // uncertainty, never an invented card
    }
    for c in &board.root {
        if c.printing == Some("30042") && c.rezzed {
            lines = prune(
                lines
                    .into_iter()
                    .flat_map(|line| {
                        let credit = line.pay(5);
                        let click = (line.clicks >= 2).then(|| {
                            let mut paid = line;
                            paid.clicks -= 2;
                            paid.cost += 4;
                            paid
                        });
                        credit.into_iter().chain(click)
                    })
                    .collect(),
            );
        }
        if c.printing == Some("30045") {
            lines.retain_mut(|line| {
                let damage = 2 + c.advancement as usize;
                if damage > line.hand {
                    return false;
                }
                line.hand -= damage;
                line.cost += damage as u32 * 2;
                true
            });
        }
    }
    lines
}
fn encounter_action(view: &Observation) -> Option<Action> {
    let run = view.run.as_ref()?;
    let mut ice = run.ice.as_ref()?.ice_mechanics(run.server)?;
    ice.strength = run.ice_strength?;
    let broken = run
        .subroutines
        .iter()
        .filter(|s| s.broken)
        .fold(0, |m, s| m | (1 << (s.number - 1)));
    let lines: Vec<_> = ice_lines(view, ice, broken, true, Line::start(view))
        .into_iter()
        .map(|mut l| {
            l.first = Some(l.first.unwrap_or(Action::PassPriority));
            l
        })
        .collect();
    let safe_exit = if matches!(
        ice.effect,
        IceEffect::Tithe | IceEffect::Karuna | IceEffect::Diviner
    ) {
        lines.iter().min_by_key(|l| l.cost).and_then(|l| l.first)
    } else {
        None
    };
    rest(view, run.server, run.position.unwrap_or(0), lines)
        .into_iter()
        .min_by_key(|l| l.cost)
        .and_then(|l| l.first)
        .or(safe_exit)
}
fn has(view: &Observation, printing: &str) -> bool {
    view.rig.iter().any(|c| c.printing == Some(printing))
}
fn keep(view: &Observation, c: &CardView) -> i32 {
    match c.printing {
        Some("30006" | "30026" | "30015") => {
            if view
                .rig
                .iter()
                .any(|r| r.printing == c.printing && r.instance != c.instance)
            {
                5
            } else {
                40
            }
        }
        Some("30032") => 15,
        Some("30030" | "30027") => 35,
        Some("30020" | "30021") => 25,
        Some("30028" | "30029") => 22,
        Some("30014" | "30013" | "30034") => {
            if view
                .rig
                .iter()
                .any(|r| r.printing == c.printing && r.instance != c.instance)
            {
                0
            } else {
                20
            }
        }
        Some("30018") => 20,
        Some("30033") => 8,
        _ => 10,
    }
}
fn target_value(view: &Observation, server: Server) -> i32 {
    let Some(s) = view.servers.iter().find(|s| s.server == server) else {
        return 0;
    };
    let base = match server {
        Server::Hq => {
            if view.hq_count == 0 {
                0
            } else {
                26 + if has(view, "30013") && !view.hq_breached_this_turn {
                    8
                } else {
                    0
                }
            }
        }
        Server::Rd => {
            if view.rd_count == 0 {
                0
            } else {
                25
            }
        }
        Server::Archives => {
            if view
                .archives
                .iter()
                .any(|c| matches!(c.printing, Some("30067" | "30069" | "30070")))
            {
                90
            } else if view.archives.iter().any(|c| c.printing.is_none()) {
                17
            } else {
                0
            }
        }
        Server::Remote(_) => s
            .root
            .iter()
            .map(|c| match c.printing {
                Some("30067" | "30069" | "30070") => 90,
                None => 38 + (c.advancement.min(3) as i32 * 2), // agenda OR trap, never inferred type
                Some("30037" | "30071") => 18,
                _ => 0,
            })
            .max()
            .unwrap_or(0),
        _ => 0,
    };
    base - if view.central_runs.contains(&server) {
        24
    } else {
        0
    }
}
fn run_value(view: &Observation, server: Server, line: Line) -> Option<i32> {
    let s = view.servers.iter().find(|s| s.server == server)?;
    let best = rest(view, server, s.ice.len(), vec![line])
        .into_iter()
        .min_by_key(|l| l.cost)?;
    let mut value = target_value(view, server) - best.cost as i32 * 3;
    // A face-down advanced remote may be Urtica. It stays uncertain; low grip
    // requires recovery before risking its publicly possible damage.
    if s.root
        .iter()
        .any(|c| c.printing.is_none() && c.advancement > 0)
        && best.hand < 5
    {
        value -= 18;
    }
    if has(view, "30014") {
        value += 2;
    }
    Some(value)
}
fn strategic_score(view: &Observation, action: Action) -> Option<i32> {
    let clicks = view.clicks[1];
    let credits = view.runner_credits;
    let hand = view.hand.len();
    match action {
        Action::GainCredit => Some(if credits < 3 {
            30
        } else if credits < 7 {
            14
        } else {
            5
        }),
        Action::Draw => Some(if view.stack_count == 0 {
            0
        } else if hand < 2 && credits > 0 {
            32
        } else if hand < 4 {
            20
        } else if hand < 5 && has(view, "30034") && !view.runner_basic_draw_taken {
            16
        } else {
            2
        }),
        Action::TakeTelework(_) => Some(if credits < 12 { 33 } else { 20 }),
        Action::TakePennyshaver(id) => card(view, id).map(|c| {
            if c.hosted_credits > 0 {
                22 + c.hosted_credits.min(10) as i32 * 3
            } else {
                if credits < 3 { 24 } else { 6 }
            }
        }),
        Action::LoadSmartware(id) => card(view, id).map(|c| {
            if c.hosted_credits < 2 && credits >= 5 {
                12
            } else {
                1
            }
        }),
        Action::RunServer(server) | Action::RunRedTeam { server, .. } => {
            let mut line = Line::start(view);
            line.clicks = line.clicks.saturating_sub(1);
            let bonus = if matches!(action, Action::RunRedTeam { .. }) {
                10
            } else {
                0
            };
            run_value(view, server, line).map(|v| v + bonus)
        }
        Action::PlayCard(id) => {
            let c = card(view, id)?;
            match c.printing {
                Some("30030") => Some(if credits < 15 { 36 } else { 23 }),
                Some("30020") => Some(if clicks == 1 {
                    38
                } else if credits < 5 {
                    30
                } else {
                    18
                }),
                Some("30021") => Some(if hand <= 2 {
                    36
                } else if hand <= 4 && clicks == 1 {
                    24
                } else {
                    2
                }),
                Some("30028" | "30029" | "30012") => {
                    let cost = u32::from(c.printing != Some("30028"));
                    let mut line = Line::start(view);
                    line.pool = line.pool.checked_sub(cost)?;
                    line.clicks = line.clicks.saturating_sub(1);
                    line.hand = line.hand.saturating_sub(1);
                    line.cost = cost;
                    if c.printing == Some("30029") {
                        line.hosted = 5;
                    }
                    view.servers
                        .iter()
                        .filter(|s| {
                            c.printing != Some("30028")
                                || matches!(s.server, Server::Hq | Server::Rd)
                        })
                        .filter_map(|s| {
                            run_value(view, s.server, line).map(|v| {
                                v + if c.printing == Some("30028") {
                                    9
                                } else if c.printing == Some("30012") {
                                    s.ice.iter().filter(|c| !c.rezzed).count() as i32 * 2
                                } else {
                                    0
                                }
                            })
                        })
                        .max()
                }
                _ => None,
            }
        }
        Action::InstallCard(id) => {
            let c = card(view, id)?;
            let print = c.printing?;
            if let Some(mechanics) = c.breaker_mechanics(1) {
                let covered = view
                    .rig
                    .iter()
                    .filter_map(|r| r.breaker_mechanics(1))
                    .any(|r| r.family == mechanics.family);
                if covered {
                    return Some(0);
                }
                let exposed = view
                    .servers
                    .iter()
                    .flat_map(|s| s.ice.iter().filter_map(|i| i.ice_mechanics(s.server)))
                    .any(|i| mechanics.family.is_none_or(|f| i.family == f));
                return Some(
                    if mechanics.disposable {
                        if view.memory_capacity - view.memory_used >= 2 {
                            18
                        } else {
                            0
                        }
                    } else if exposed {
                        48
                    } else {
                        27
                    } + if print == "30015" && view.successful_run_this_turn {
                        4
                    } else {
                        0
                    },
                );
            }
            Some(match print {
                "30027" => {
                    if credits < 10 {
                        34
                    } else {
                        21
                    }
                }
                "30014" => {
                    if credits >= 6 {
                        26
                    } else {
                        9
                    }
                }
                "30018" => {
                    if credits >= 8 {
                        25
                    } else {
                        5
                    }
                }
                "30034" => {
                    if view.stack_count > 8 && credits >= 7 {
                        23
                    } else {
                        4
                    }
                }
                "30013" => {
                    if credits >= 7 {
                        18
                    } else {
                        6
                    }
                }
                "30033" if !has(view, print) => 10,
                _ => 0,
            })
        }
        _ => None,
    }
}
pub(super) fn response(view: &Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if choices.contains(&Action::KeepHand) {
        let economy = view
            .hand
            .iter()
            .any(|c| matches!(c.printing, Some("30030" | "30020" | "30027")));
        return Some(if economy {
            Action::KeepHand
        } else {
            Action::Mulligan
        });
    }
    for action in [
        Action::StealAgenda,
        Action::ConfirmDiscard,
        Action::FinishInstall,
    ] {
        if choices.contains(&action) {
            return Some(action);
        }
    }
    if let Some(action) = choices
        .iter()
        .filter_map(|a| match a {
            Action::ToggleDiscard(id) => card(view, *id)
                .filter(|c| !c.selected)
                .map(|c| (*a, keep(view, c))),
            Action::TrashInstalled(id) => card(view, *id).map(|c| (*a, keep(view, c))),
            _ => None,
        })
        .min_by_key(|a| a.1)
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
        .filter_map(|a| {
            if let Action::PayRunCredits(n) = a {
                Some((*n, *a))
            } else {
                None
            }
        })
        .max_by_key(|p| p.0)
    {
        return Some(action.1);
    }
    if let Some(run) = &view.run {
        if choices.contains(&Action::RefuseManegarm) {
            if view
                .servers
                .iter()
                .find(|s| s.server == run.server)
                .is_some_and(|s| {
                    s.root.iter().any(|c| {
                        c.printing == Some("30045") && 2 + c.advancement as usize > view.hand.len()
                    })
                })
            {
                return Some(Action::RefuseManegarm);
            }
            // Five temporary credits are cheaper than two future click actions.
            let credit_cost = 5u32.saturating_sub(u32::from(run.hosted_credits));
            for action in if credit_cost < 4 {
                [Action::PayManegarmCredits, Action::SpendManegarmClicks]
            } else {
                [Action::SpendManegarmClicks, Action::PayManegarmCredits]
            } {
                if choices.contains(&action) {
                    return Some(action);
                }
            }
            return Some(Action::RefuseManegarm);
        }
        if choices.contains(&Action::Continue) {
            let mut line = Line::start(view);
            if run.karuna_continuation && run.subroutines.get(1).is_some_and(|s| !s.broken) {
                if line.hand < 2 {
                    return Some(Action::JackOut);
                }
                line.hand -= 2;
            }
            return Some(
                if rest(view, run.server, run.position.unwrap_or(0), vec![line]).is_empty() {
                    Action::JackOut
                } else {
                    Action::Continue
                },
            );
        }
        if run.encounter_paid {
            return Some(
                encounter_action(view)
                    .filter(|a| choices.contains(a))
                    .unwrap_or(Action::PassPriority),
            );
        }
        if choices.contains(&Action::FinishAccess) {
            let useful = view
                .accessed
                .as_ref()
                .is_some_and(|c| matches!(c.printing, Some("30037" | "30071" | "30042")));
            return Some(if useful && choices.contains(&Action::TrashAccessed) {
                Action::TrashAccessed
            } else {
                Action::FinishAccess
            });
        }
        for action in [Action::AccessRd, Action::AccessHq] {
            if choices.contains(&action) {
                return Some(action);
            }
        }
        if let Some(action) = choices
            .iter()
            .filter_map(|a| match a {
                Action::AccessArchives(i) => Some((
                    *a,
                    view.archives.get(*i).map_or(0, |c| {
                        if matches!(c.printing, Some("30067" | "30069" | "30070")) {
                            100
                        } else {
                            0
                        }
                    }),
                )),
                Action::AccessRoot(i) => Some((
                    *a,
                    view.servers
                        .iter()
                        .find(|s| s.server == run.server)
                        .and_then(|s| s.root.get(*i))
                        .map_or(0, |c| match c.printing {
                            Some("30067" | "30069" | "30070") => 100,
                            Some("30045") => -100,
                            _ => 0,
                        }),
                )),
                _ => None,
            })
            .max_by_key(|a| a.1)
        {
            return Some(action.0);
        }
        return choices
            .contains(&Action::PassPriority)
            .then_some(Action::PassPriority);
    }
    // Event target selection has already paid its click/card. Recover only the
    // lawful public event in play, never rules text or a private continuation.
    if !choices.contains(&Action::GainCredit)
        && choices.iter().any(|a| matches!(a, Action::RunServer(_)))
    {
        let mut line = Line::start(view);
        if view.play_area.iter().any(|c| c.printing == Some("30029")) {
            line.hosted = 5;
        }
        return choices
            .iter()
            .filter_map(|a| {
                if let Action::RunServer(server) = a {
                    Some((*a, run_value(view, *server, line).unwrap_or(-1000)))
                } else {
                    None
                }
            })
            .max_by_key(|a| a.1)
            .map(|a| a.0);
    }
    choices
        .iter()
        .filter_map(|a| strategic_score(view, *a).map(|score| (*a, score)))
        .max_by_key(|a| a.1)
        .map(|a| a.0)
        .or_else(|| {
            choices
                .contains(&Action::PassPriority)
                .then_some(Action::PassPriority)
        })
}
