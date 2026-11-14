//! Concrete admitted ICE continuations; CR 26.03 6.2, 6.9, 8.5, 9.5.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SubroutineView {
    pub number: u8,
    pub text: &'static str,
    pub broken: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RunView {
    pub server: Server,
    pub encounter_paid: bool,
    pub server_approach: bool,
    pub ice: Option<CardView>,
    pub ice_strength: Option<u64>,
    pub subroutines: Vec<SubroutineView>,
    pub breakers: Vec<(CardId, String, u64)>,
    pub hosted_credits: u8,
    pub payment_cost: Option<u8>,
    pub payment_for: Option<String>,
    pub overclock: Option<CardView>,
    pub position: Option<usize>,
    pub hq_access_remaining: Option<u8>,
    pub known_hq: Vec<CardView>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(in crate::game) struct ActiveRun {
    server: Server,
    positions: Vec<Option<CardId>>,
    cursor: Option<usize>,
    broken: u8,
    boosts: Vec<(CardId, u64)>,
    pub(super) overclock: Option<CardId>,
    pub(super) hosted: u8,
    stage: Stage,
    accessed: Vec<CardId>,
    hq_remaining: Option<u8>,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
enum OtherWindow {
    Initiation,
    BeforeMovement,
    AfterMovement,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
enum PaymentWindow {
    Approach,
    Encounter,
    Other(OtherWindow),
    Server,
    Access,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
enum Payment {
    Boost(CardId),
    Break(CardId, u8),
    Manegarm(CardId),
    Trash(CardId),
}
impl Payment {
    fn description(self, state: &State) -> String {
        match self {
            Self::Trash(_) => "trash Manegarm Skunkworks".into(),
            Self::Manegarm(_) => "Manegarm: continue the run".into(),
            Self::Boost(id) => format!("{}: boost strength", state.card(id).kind.description().0),
            Self::Break(id, mask) => format!(
                "{}: break subroutine {}",
                state.card(id).kind.description().0,
                (0..3)
                    .filter(|n| mask & (1 << n) != 0)
                    .map(|n| (n + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(" + ")
            ),
        }
    }
}
#[derive(Clone, Copy, Serialize, Deserialize)]
enum Stage {
    OtherPaid {
        point: OtherWindow,
        priority: Player,
        passed: bool,
    },
    Approach {
        priority: Player,
        passed: bool,
    },
    Encounter {
        priority: Player,
        passed: bool,
    },
    Payment {
        effect: Payment,
        cost: u8,
        resume: PaymentWindow,
    },
    BranInstall {
        source: CardId,
    },
    BranTrash {
        source: CardId,
        card: CardId,
        inward: usize,
    },
    Movement,
    ServerBoundary,
    Manegarm {
        source: CardId,
    },
    Breach,
    Access {
        card: CardId,
    },
}
impl State {
    pub(super) fn run_knows(&self, id: CardId, viewer: Player) -> bool {
        viewer == Player::Runner
            && self
                .turn()
                .run
                .as_ref()
                .is_some_and(|r| r.accessed.contains(&id))
    }
    pub(super) fn accessed_in_run(&self, viewer: Player) -> Option<CardView> {
        let run = self.turn().run.as_ref()?;
        let id = match run.stage {
            Stage::Access { card }
            | Stage::Payment {
                effect: Payment::Trash(card),
                ..
            } => card,
            _ => return None,
        };
        let mut view = self.turn_view(self.card(id), viewer);
        // Access identity is not a persistent identifier for a hidden HQ object.
        view.instance = None;
        Some(view)
    }
    fn breach_hq(&mut self) {
        if self.run().server != Server::Hq {
            self.run_pending(Stage::ServerBoundary);
            return;
        }
        let extra = !self.turn().hq_breached
            && self
                .cards
                .iter()
                .any(|c| c.kind == CardKind::DocklandsPass && c.zone == Zone::Rig);
        self.turn_mut().hq_breached = true;
        self.run_mut().hq_remaining = Some(if extra { 2 } else { 1 });
        self.log(format!(
            "Successful run: breach HQ, accessing up to {} HQ card{} plus root cards{}.",
            if extra { 2 } else { 1 },
            if extra { "s" } else { "" },
            if extra {
                " (Docklands Pass: first HQ breach this turn)"
            } else {
                ""
            }
        ));
        self.finish_access();
    }
    fn hq_candidates(&self) -> Vec<CardId> {
        self.cards
            .iter()
            .filter(|c| c.zone == Zone::Hq && !self.run().accessed.contains(&c.id))
            .map(|c| c.id)
            .collect()
    }
    fn breach_choices(&self) -> Vec<Action> {
        let mut out: Vec<_> = self
            .cards
            .iter()
            .filter(|c| c.zone == Zone::Root(self.run().server))
            .enumerate()
            .filter(|(_, c)| !self.run().accessed.contains(&c.id))
            .map(|(index, _)| Action::AccessRoot(index))
            .collect();
        if self.run().hq_remaining.unwrap_or(0) > 0 && !self.hq_candidates().is_empty() {
            out.push(Action::AccessHq);
        }
        out
    }
    fn begin_access(&mut self, id: CardId) {
        self.run_mut().accessed.push(id);
        let title = self.card(id).kind.description().0;
        self.history.push([
            format!("Runner accesses {title}."),
            format!("Runner accesses {title}."),
        ]);
        self.run_pending(Stage::Access { card: id });
    }
    fn finish_access(&mut self) {
        if self.breach_choices().is_empty() {
            self.end_run(RunResult::SuccessfulAccess);
        } else {
            self.run_pending(Stage::Breach);
        }
    }

    pub(super) fn non_ice_rez_choices(&self) -> Vec<Action> {
        self.cards
            .iter()
            .filter(|c| {
                c.kind == CardKind::ManegarmSkunkworks
                    && matches!(c.zone, Zone::Root(_))
                    && !c.rezzed
                    && self.corp_credits >= 2
            })
            .map(|c| Action::Rez(c.id))
            .collect()
    }
    pub(super) fn rez_non_ice(&mut self, id: CardId) {
        self.corp_credits -= 2;
        self.card_mut(id).rezzed = true;
        self.card_mut(id).faceup = true;
        self.log("Corp rezzes Manegarm Skunkworks for 2 credits.".into());
    }
    fn approach_server(&mut self) {
        if let Some(source) = self
            .cards
            .iter()
            .find(|c| {
                c.zone == Zone::Root(self.run().server)
                    && c.kind == CardKind::ManegarmSkunkworks
                    && c.rezzed
            })
            .map(|c| c.id)
        {
            self.run_pending(Stage::Manegarm { source });
        } else {
            self.breach_hq();
        }
    }

    pub(super) fn run_installing(&self) -> bool {
        self.turn()
            .run
            .as_ref()
            .is_some_and(|r| matches!(r.stage, Stage::BranTrash { .. }))
    }
    pub(super) fn run_phase(&self) -> &'static str {
        match self.turn().run.as_ref().unwrap().stage {
            Stage::OtherPaid {
                point: OtherWindow::Initiation,
                ..
            } => "Run begins · paid window",
            Stage::OtherPaid {
                point: OtherWindow::BeforeMovement,
                ..
            } => "ICE passed · paid window",
            Stage::OtherPaid {
                point: OtherWindow::AfterMovement,
                ..
            } => "Moving inward · paid window",
            Stage::Approach { .. } => "Approach ICE · paid / rez window",
            Stage::Encounter { .. } => "Encounter · break subroutines before passing",
            Stage::Payment { .. } => "Choose pool / Overclock payment",
            Stage::BranInstall { .. } => "Brân · optional inward installation",
            Stage::BranTrash { .. } => "Installation · optional trash, then finish",
            Stage::Movement => "Movement · continue or jack out",
            Stage::Breach => "Breach HQ · choose root or random HQ access",
            Stage::Access { card } => {
                if self.card(card).kind == CardKind::OffworldOffice {
                    "Accessing Offworld Office · must steal"
                } else if self.card(card).kind == CardKind::ManegarmSkunkworks {
                    "Accessing Manegarm · optional trash for 3 credits"
                } else {
                    "Access complete · leave card"
                }
            }
            Stage::Manegarm { .. } => {
                "Manegarm · pay 5 credits or spend 2 clicks, otherwise end the run"
            }
            Stage::ServerBoundary => "Server approach · continuation unavailable",
        }
    }
    fn run(&self) -> &ActiveRun {
        self.turn().run.as_ref().unwrap()
    }
    fn run_mut(&mut self) -> &mut ActiveRun {
        self.turn_mut().run.as_mut().unwrap()
    }
    fn current_ice(&self) -> Option<CardId> {
        self.run().cursor.and_then(|i| self.run().positions[i])
    }
    fn run_pending(&mut self, stage: Stage) {
        let actor = match stage {
            Stage::OtherPaid { priority, .. }
            | Stage::Approach { priority, .. }
            | Stage::Encounter { priority, .. } => priority,
            Stage::BranInstall { .. } | Stage::BranTrash { .. } => Player::Corp,
            _ => Player::Runner,
        };
        self.run_mut().stage = stage;
        self.pending = Some(Pending::RunStep { actor });
    }
    fn strength(&self, id: CardId) -> u64 {
        let base = match self.card(id).kind {
            CardKind::Bran => 6,
            CardKind::Palisade => {
                if matches!(self.run().server, Server::Remote(_)) {
                    4
                } else {
                    2
                }
            }
            CardKind::Cleaver => 3,
            CardKind::Unity => 1,
            _ => 0,
        };
        base + self
            .run()
            .boosts
            .iter()
            .find(|(card, _)| *card == id)
            .map_or(0, |(_, n)| *n)
    }
    pub(super) fn run_view(&self, viewer: Player) -> Option<RunView> {
        let run = self.turn().run.as_ref()?;
        let ice = self.current_ice().map(|id| self.card(id));
        let revealed = ice.filter(|c| c.rezzed);
        let subroutines = revealed.map_or(vec![], |c| {
            let texts: &[&'static str] = match c.kind {
                CardKind::Bran => &[
                    "Optional inward ICE installation",
                    "End the run",
                    "End the run",
                ],
                CardKind::Palisade => &["End the run"],
                CardKind::Whitespace => &[
                    "Lose 3 pool credits",
                    "End the run if pool has 6 credits or less",
                ],
                _ => &[],
            };
            texts
                .iter()
                .enumerate()
                .map(|(i, &text)| SubroutineView {
                    number: i as u8 + 1,
                    text,
                    broken: run.broken & (1 << i) != 0,
                })
                .collect()
        });
        Some(RunView {
            server: run.server,
            hq_access_remaining: run.hq_remaining,
            // Preserve the publicly witnessed access order, never the private
            // backing order of HQ or its instance identifiers.
            known_hq: run
                .accessed
                .iter()
                .map(|id| self.card(*id))
                .filter(|c| c.zone == Zone::Hq)
                .map(|c| {
                    let mut view = self.turn_view(c, viewer);
                    view.instance = None;
                    view
                })
                .collect(),
            server_approach: matches!(run.stage, Stage::ServerBoundary | Stage::Manegarm { .. }),
            encounter_paid: matches!(
                run.stage,
                Stage::Encounter { .. }
                    | Stage::Payment {
                        resume: PaymentWindow::Encounter,
                        ..
                    }
            ),
            ice: ice.map(|c| self.turn_view(c, viewer)),
            ice_strength: revealed.map(|c| self.strength(c.id)),
            subroutines,
            breakers: self
                .cards
                .iter()
                .filter(|c| c.zone == Zone::Rig && c.kind.memory() > 0)
                .map(|c| (c.id, c.kind.description().0.into(), self.strength(c.id)))
                .collect(),
            hosted_credits: run.hosted,
            payment_cost: if let Stage::Payment { cost, .. } = run.stage {
                Some(cost)
            } else {
                None
            },
            payment_for: if let Stage::Payment { effect, .. } = run.stage {
                Some(effect.description(self))
            } else {
                None
            },
            overclock: run
                .overclock
                .map(|id| self.turn_view(self.card(id), viewer)),
            position: run
                .cursor
                .map(|i| run.positions[..i].iter().flatten().count()),
        })
    }
    pub(super) fn boost_choices(&self, hosted: u8) -> Vec<Action> {
        self.cards
            .iter()
            .filter(|c| {
                c.zone == Zone::Rig
                    && c.kind.memory() > 0
                    && self.turn().runner_credits + u32::from(hosted)
                        >= if c.kind == CardKind::Cleaver { 2 } else { 1 }
            })
            .map(|c| Action::Boost(c.id))
            .collect()
    }
    pub(super) fn run_choices(&self) -> Vec<Action> {
        if matches!(self.pending, Some(Pending::RunServer)) {
            return self.servers().into_iter().map(Action::RunServer).collect();
        }
        let run = self.run();
        match run.stage {
            Stage::OtherPaid {
                priority, point, ..
            } => {
                let mut out = if priority == Player::Runner {
                    self.boost_choices(run.hosted)
                } else {
                    vec![]
                };
                if priority == Player::Corp && matches!(point, OtherWindow::AfterMovement) {
                    out.extend(self.non_ice_rez_choices());
                }
                out.push(Action::PassPriority);
                out
            }
            Stage::Approach { priority, .. } => {
                let mut out = if priority == Player::Runner {
                    self.boost_choices(run.hosted)
                } else {
                    vec![]
                };
                if priority == Player::Corp {
                    out.extend(self.non_ice_rez_choices());
                }
                if priority == Player::Corp
                    && let Some(id) = self.current_ice()
                    && !self.card(id).rezzed
                    && self.corp_credits >= self.rez_cost(id)
                {
                    out.push(Action::Rez(id));
                }
                out.push(Action::PassPriority);
                out
            }
            Stage::Encounter { priority, .. } => {
                let mut out = vec![];
                if priority == Player::Runner {
                    let ice = self.current_ice().unwrap();
                    let kind = self.card(ice).kind;
                    let count = match kind {
                        CardKind::Bran => 3,
                        CardKind::Whitespace => 2,
                        _ => 1,
                    };
                    if kind == CardKind::Bran && self.turn().clicks[1] > 0 {
                        for n in 0..3 {
                            if run.broken & (1 << n) == 0 {
                                out.push(Action::BreakWithClick(n + 1));
                            }
                        }
                    }
                    for card in self
                        .cards
                        .iter()
                        .filter(|c| c.zone == Zone::Rig && c.kind.memory() > 0)
                    {
                        let boost_cost = if card.kind == CardKind::Cleaver { 2 } else { 1 };
                        if self.turn().runner_credits + u32::from(run.hosted) >= boost_cost {
                            out.push(Action::Boost(card.id));
                        }
                        let compatible = match card.kind {
                            CardKind::Cleaver => {
                                matches!(kind, CardKind::Bran | CardKind::Palisade)
                            }
                            CardKind::Unity => kind == CardKind::Whitespace,
                            _ => false,
                        };
                        if compatible
                            && self.strength(card.id) >= self.strength(ice)
                            && self.turn().runner_credits + u32::from(run.hosted) >= 1
                        {
                            for mask in 1u8..(1 << count) {
                                if mask & run.broken == 0
                                    && mask.count_ones()
                                        <= if card.kind == CardKind::Cleaver { 2 } else { 1 }
                                {
                                    out.push(Action::Break {
                                        breaker: card.id,
                                        subroutines: mask,
                                    });
                                }
                            }
                        }
                    }
                }
                out.push(Action::PassPriority);
                out
            }
            Stage::Payment { cost, .. } => (0..=cost.min(run.hosted))
                .filter(|&hosted| u32::from(cost - hosted) <= self.turn().runner_credits)
                .map(Action::PayRunCredits)
                .collect(),
            Stage::BranInstall { .. } => {
                let mut out = vec![Action::DeclineInstall];
                out.extend(
                    self.cards
                        .iter()
                        .filter(|c| c.kind.ice() && matches!(c.zone, Zone::Hq | Zone::Archives))
                        .map(|c| Action::InstallInward(c.id)),
                );
                out
            }
            Stage::BranTrash { .. } => {
                let mut out: Vec<_> = run
                    .positions
                    .iter()
                    .flatten()
                    .copied()
                    .map(Action::TrashInstalled)
                    .collect();
                out.push(Action::FinishInstall);
                out
            }
            Stage::Movement => vec![Action::Continue, Action::JackOut],
            Stage::Breach => self.breach_choices(),
            Stage::Access { card } => match self.card(card).kind {
                CardKind::OffworldOffice => vec![Action::StealAgenda],
                CardKind::ManegarmSkunkworks => {
                    let mut out = vec![];
                    if self.turn().runner_credits + u32::from(run.hosted) >= 3 {
                        out.push(Action::TrashAccessed);
                    }
                    out.push(Action::FinishAccess);
                    out
                }
                _ => vec![Action::FinishAccess],
            },
            Stage::Manegarm { .. } => {
                let mut out = vec![];
                if self.turn().runner_credits + u32::from(run.hosted) >= 5 {
                    out.push(Action::PayManegarmCredits);
                }
                if self.turn().clicks[1] >= 2 {
                    out.push(Action::SpendManegarmClicks);
                }
                out.push(Action::RefuseManegarm);
                out
            }
            Stage::ServerBoundary => vec![],
        }
    }
    fn rez_cost(&self, id: CardId) -> u32 {
        match self.card(id).kind {
            CardKind::Bran => 6,
            CardKind::Palisade => 3,
            CardKind::Whitespace | CardKind::ManegarmSkunkworks => 2,
            _ => 0,
        }
    }
    fn encounter(&mut self) {
        self.run_mut().broken = 0;
        self.run_mut().boosts.clear();
        self.run_pending(Stage::Encounter {
            priority: Player::Runner,
            passed: false,
        });
    }
    fn resolve_subroutines(&mut self, start: u8) {
        let Some(id) = self.current_ice() else {
            self.movement();
            return;
        };
        let kind = self.card(id).kind;
        let count = match kind {
            CardKind::Bran => 3,
            CardKind::Whitespace => 2,
            _ => 1,
        };
        for n in start..count {
            if self.run().broken & (1 << n) != 0 {
                continue;
            }
            match (kind, n) {
                (CardKind::Bran, 0) => {
                    self.run_pending(Stage::BranInstall { source: id });
                    return;
                }
                (CardKind::Whitespace, 0) => {
                    self.turn_mut().runner_credits = self.turn().runner_credits.saturating_sub(3);
                    self.log("Whitespace: Runner loses 3 pool credits (or all remaining). Overclock credits are unaffected.".into());
                }
                (CardKind::Whitespace, 1) if self.turn().runner_credits > 6 => {}
                _ => {
                    self.end_run(RunResult::EndedByIce);
                    return;
                }
            }
        }
        self.movement();
    }
    fn movement(&mut self) {
        self.run_mut().boosts.clear();
        self.run_pending(Stage::OtherPaid {
            point: OtherWindow::BeforeMovement,
            priority: Player::Runner,
            passed: false,
        });
    }
    fn end_run(&mut self, result: RunResult) {
        self.log(format!(
            "Run ends: {result:?}. Unspent Overclock credits return to the bank."
        ));
        let overclock = self.run().overclock;
        self.turn_mut().run = None;
        self.run_result = Some(result);
        if let Some(id) = overclock {
            self.trash(id);
        }
        self.finish_action();
    }
    pub(super) fn apply_run_choice(
        &mut self,
        actor: Player,
        action: Action,
    ) -> Result<(), &'static str> {
        if let Action::RunServer(server) = action {
            if matches!(self.pending, Some(Pending::Action { .. })) {
                self.turn_mut().clicks[1] -= 1;
            }
            let positions: Vec<_> = self
                .cards
                .iter()
                .filter(|c| c.kind.ice() && self.card_server(c) == Some(server))
                .map(|c| Some(c.id))
                .collect();
            let cursor = positions.len().checked_sub(1);
            let overclock = self
                .cards
                .iter()
                .find(|c| c.kind == CardKind::Overclock && c.zone == Zone::PlayArea)
                .map(|c| c.id);
            self.turn_mut().run = Some(ActiveRun {
                server,
                positions,
                cursor,
                broken: 0,
                boosts: vec![],
                overclock,
                hosted: if overclock.is_some() { 5 } else { 0 },
                stage: Stage::Movement,
                accessed: vec![],
                hq_remaining: None,
            });
            self.run_result = None;
            self.log(format!(
                "Runner runs {}. {} Overclock credits are separate from the pool.",
                server.label(),
                self.run().hosted
            ));
            self.run_pending(Stage::OtherPaid {
                point: OtherWindow::Initiation,
                priority: Player::Runner,
                passed: false,
            });
            return Ok(());
        }
        if let Action::Boost(id) = action {
            let resume = match self.run().stage {
                Stage::Approach { .. } => PaymentWindow::Approach,
                Stage::Encounter { .. } => PaymentWindow::Encounter,
                Stage::OtherPaid { point, .. } => PaymentWindow::Other(point),
                _ => return Err("No boost window"),
            };
            self.run_pending(Stage::Payment {
                effect: Payment::Boost(id),
                cost: if self.card(id).kind == CardKind::Cleaver {
                    2
                } else {
                    1
                },
                resume,
            });
            return Ok(());
        }
        match (self.run().stage, action) {
            (Stage::Breach, Action::AccessRoot(index)) => {
                let id = self
                    .cards
                    .iter()
                    .filter(|c| c.zone == Zone::Root(self.run().server))
                    .nth(index)
                    .unwrap()
                    .id;
                self.begin_access(id);
            }
            (Stage::Breach, Action::AccessHq) => {
                let candidates = self.hq_candidates();
                let mut random = SeededRandom::new(self.turn().random_state);
                let id = candidates[random.index(candidates.len())];
                self.turn_mut().random_state = random.state();
                self.run_mut().hq_remaining = self.run().hq_remaining.map(|n| n - 1);
                self.begin_access(id);
            }
            (Stage::Access { .. }, Action::FinishAccess) => self.finish_access(),
            (Stage::Access { card }, Action::TrashAccessed) => self.run_pending(Stage::Payment {
                effect: Payment::Trash(card),
                cost: 3,
                resume: PaymentWindow::Access,
            }),
            (Stage::Access { card }, Action::StealAgenda) => {
                let c = self.card_mut(card);
                c.zone = Zone::RunnerScore;
                c.advancement = 0;
                c.faceup = true;
                c.installed_turn = None;
                c.installed_by = None;
                self.turn_mut().points[1] += 2;
                self.log("Runner steals Offworld Office for 2 agenda points. Corp receives no scoring reward.".into());
                self.win_checkpoint();
                if self.turn().winner.is_none() {
                    self.finish_access();
                }
            }

            (Stage::Manegarm { source }, Action::PayManegarmCredits) => {
                self.run_pending(Stage::Payment {
                    effect: Payment::Manegarm(source),
                    cost: 5,
                    resume: PaymentWindow::Server,
                })
            }
            (Stage::Manegarm { .. }, Action::SpendManegarmClicks) => {
                self.turn_mut().clicks[1] -= 2;
                self.log("Runner spends 2 clicks for Manegarm; the run continues.".into());
                self.breach_hq();
            }
            (Stage::Manegarm { .. }, Action::RefuseManegarm) => {
                self.end_run(RunResult::EndedByManegarm)
            }
            (
                Stage::OtherPaid {
                    point: OtherWindow::AfterMovement,
                    ..
                },
                Action::Rez(id),
            ) => {
                self.rez_non_ice(id);
                self.run_pending(Stage::OtherPaid {
                    point: OtherWindow::AfterMovement,
                    priority: actor,
                    passed: false,
                });
            }
            (Stage::OtherPaid { point, passed, .. }, Action::PassPriority) => {
                if !passed {
                    self.run_pending(Stage::OtherPaid {
                        point,
                        priority: actor.other(),
                        passed: true,
                    });
                } else {
                    match point {
                        OtherWindow::Initiation => {
                            if self.run().cursor.is_some() {
                                self.run_pending(Stage::Approach {
                                    priority: Player::Runner,
                                    passed: false,
                                })
                            } else {
                                self.movement()
                            }
                        }
                        OtherWindow::BeforeMovement => self.run_pending(Stage::Movement),
                        OtherWindow::AfterMovement => {
                            if self.run().cursor.is_some() {
                                self.run_pending(Stage::Approach {
                                    priority: Player::Runner,
                                    passed: false,
                                });
                            } else {
                                self.approach_server();
                            }
                        }
                    }
                }
            }
            (Stage::Approach { passed, .. }, Action::PassPriority) => {
                if passed {
                    if self.current_ice().is_some_and(|id| self.card(id).rezzed) {
                        self.encounter()
                    } else {
                        self.movement()
                    }
                } else {
                    self.run_pending(Stage::Approach {
                        priority: actor.other(),
                        passed: true,
                    })
                }
            }
            (Stage::Approach { .. }, Action::Rez(id)) => {
                let cost = self.rez_cost(id);
                self.corp_credits -= cost;
                self.card_mut(id).rezzed = true;
                self.card_mut(id).faceup = true;
                self.log(format!(
                    "Corp rezzes {} for {cost} credits.",
                    self.card(id).kind.description().0
                ));
                self.run_pending(Stage::Approach {
                    priority: actor,
                    passed: false,
                });
            }
            (Stage::Encounter { passed, .. }, Action::PassPriority) => {
                if passed {
                    self.resolve_subroutines(0)
                } else {
                    self.run_pending(Stage::Encounter {
                        priority: actor.other(),
                        passed: true,
                    })
                }
            }
            (Stage::Encounter { .. }, Action::BreakWithClick(n)) => {
                self.turn_mut().clicks[1] -= 1;
                self.run_mut().broken |= 1 << (n - 1);
                self.log(format!(
                    "Runner loses 1 click to break Brân subroutine {n}."
                ));
                self.run_pending(Stage::Encounter {
                    priority: Player::Runner,
                    passed: false,
                });
            }
            (
                Stage::Encounter { .. },
                Action::Break {
                    breaker,
                    subroutines,
                },
            ) => self.run_pending(Stage::Payment {
                effect: Payment::Break(breaker, subroutines),
                cost: 1,
                resume: PaymentWindow::Encounter,
            }),
            (
                Stage::Payment {
                    effect,
                    cost,
                    resume,
                },
                Action::PayRunCredits(hosted),
            ) => {
                self.run_mut().hosted -= hosted;
                self.turn_mut().runner_credits -= u32::from(cost - hosted);
                self.log(format!(
                    "Runner pays {} pool + {hosted} Overclock credits for {}{}.",
                    cost - hosted,
                    effect.description(self),
                    if matches!(effect, Payment::Boost(_))
                        && !matches!(resume, PaymentWindow::Encounter)
                    {
                        " (expires immediately)"
                    } else {
                        ""
                    }
                ));
                match effect {
                    Payment::Boost(id) => {
                        let amount = if !matches!(resume, PaymentWindow::Encounter) {
                            0
                        } else if self.card(id).kind == CardKind::Unity {
                            self.cards
                                .iter()
                                .filter(|c| c.zone == Zone::Rig && c.kind.memory() > 0)
                                .count() as u64
                        } else {
                            1
                        };
                        if let Some((_, n)) = self
                            .run_mut()
                            .boosts
                            .iter_mut()
                            .find(|(card, _)| *card == id)
                        {
                            *n += amount
                        } else {
                            self.run_mut().boosts.push((id, amount));
                        }
                    }
                    Payment::Break(_, mask) => self.run_mut().broken |= mask,
                    Payment::Manegarm(_) => {
                        self.breach_hq();
                        return Ok(());
                    }
                    Payment::Trash(id) => {
                        self.card_mut(id).faceup = true;
                        self.trash(id);
                        self.log("Runner trashes Manegarm Skunkworks.".into());
                        self.finish_access();
                        return Ok(());
                    }
                }
                self.run_pending(match resume {
                    PaymentWindow::Access => unreachable!(),
                    PaymentWindow::Server => Stage::ServerBoundary,
                    PaymentWindow::Encounter => Stage::Encounter {
                        priority: Player::Runner,
                        passed: false,
                    },
                    PaymentWindow::Approach => Stage::Approach {
                        priority: Player::Runner,
                        passed: false,
                    },
                    PaymentWindow::Other(point) => Stage::OtherPaid {
                        point,
                        priority: Player::Runner,
                        passed: false,
                    },
                });
            }
            (Stage::BranInstall { .. }, Action::DeclineInstall) => self.resolve_subroutines(1),
            (Stage::BranInstall { source }, Action::InstallInward(card)) => {
                let inward = self.run().cursor.unwrap();
                self.run_mut().positions.insert(inward, None);
                self.run_mut().cursor = Some(inward + 1);
                self.card_mut(card).zone = Zone::Installing;
                self.history.push([
                    format!(
                        "Corp installs {} directly inward, ignoring costs.",
                        self.card(card).kind.description().0
                    ),
                    "Corp installs a facedown ICE directly inward, ignoring costs.".into(),
                ]);
                self.run_pending(Stage::BranTrash {
                    source,
                    card,
                    inward,
                });
            }
            (Stage::BranTrash { .. }, Action::TrashInstalled(id)) => {
                let index = self
                    .run()
                    .positions
                    .iter()
                    .position(|&c| c == Some(id))
                    .unwrap();
                self.trash(id);
                self.run_mut().positions[index] = None;
                self.log("Corp trashes ICE during Brân's installation.".into());
            }
            (
                Stage::BranTrash {
                    source,
                    card,
                    inward,
                },
                Action::FinishInstall,
            ) => {
                let number = self.turn().number;
                let server = self.run().server;
                let c = self.card_mut(card);
                c.zone = Zone::Protecting(server);
                c.rezzed = false;
                c.faceup = false;
                c.installed_turn = Some(number);
                c.installed_by = Some(Player::Corp);
                c.advancement = 0;
                self.run_mut().positions[inward] = Some(card);
                // Reorder physical ICE to match surviving stable positions.
                let ids: Vec<_> = self.run().positions.iter().flatten().copied().collect();
                let mut ordered = vec![];
                for id in ids {
                    let index = self.cards.iter().position(|c| c.id == id).unwrap();
                    ordered.push(self.cards.remove(index));
                }
                self.cards.extend(ordered);
                self.log(
                    "Inward installation completes unrezzed; a later rez costs its normal amount."
                        .into(),
                );
                if self.card(source).zone == Zone::Archives {
                    self.movement()
                } else {
                    self.resolve_subroutines(1)
                }
            }
            (Stage::Movement, Action::JackOut) => self.end_run(RunResult::JackedOut),
            (Stage::Movement, Action::Continue) => {
                let cursor = self
                    .run()
                    .cursor
                    .and_then(|i| (0..i).rev().find(|&n| self.run().positions[n].is_some()));
                // CR 6.2.4: vacant positions cease to exist once the Runner
                // leaves them. Preserve the selected inward card while pruning.
                let next = cursor.and_then(|i| self.run().positions[i]);
                self.run_mut().positions.retain(Option::is_some);
                self.run_mut().cursor =
                    next.and_then(|id| self.run().positions.iter().position(|&c| c == Some(id)));
                self.run_mut().broken = 0;
                self.run_pending(Stage::OtherPaid {
                    point: OtherWindow::AfterMovement,
                    priority: Player::Runner,
                    passed: false,
                });
            }
            _ => return Err("Unsupported run choice"),
        }
        Ok(())
    }
    fn valid_manegarm(&self, source: CardId) -> bool {
        self.run().cursor.is_none()
            && self.cards.iter().any(|c| {
                c.id == source
                    && c.kind == CardKind::ManegarmSkunkworks
                    && c.rezzed
                    && c.zone == Zone::Root(self.run().server)
            })
    }
    pub(super) fn valid_run(&self) -> bool {
        if self.turn().active != Player::Runner {
            return false;
        }
        if matches!(self.pending, Some(Pending::RunServer)) {
            return self.turn().run.is_none()
                && self
                    .cards
                    .iter()
                    .any(|c| c.kind == CardKind::Overclock && c.zone == Zone::PlayArea);
        }
        let Some(run) = &self.turn().run else {
            return false;
        };
        if run.accessed.iter().enumerate().any(|(i, id)| {
            run.accessed[..i].contains(id) || !self.cards.iter().any(|c| c.id == *id)
        }) || run.hq_remaining.is_some_and(|n| n > 2)
        {
            return false;
        }
        if run.hosted > 5
            || (run.overclock.is_none() && run.hosted != 0)
            || run.overclock.is_some_and(|id| {
                !self.cards.iter().any(|c| {
                    c.id == id && c.kind == CardKind::Overclock && c.zone == Zone::PlayArea
                })
            })
            || run.cursor.is_some_and(|i| i >= run.positions.len())
        {
            return false;
        }
        for (i, id) in run.positions.iter().enumerate() {
            if let Some(id) = id
                && (run.positions[..i].contains(&Some(*id))
                    || !self.cards.iter().any(|c| {
                        c.id == *id && c.kind.ice() && self.card_server(c) == Some(run.server)
                    }))
            {
                return false;
            }
        }
        if self.cards.iter().any(|c| {
            c.kind.ice()
                && self.card_server(c) == Some(run.server)
                && !run.positions.contains(&Some(c.id))
        }) {
            return false;
        }
        let count = self.current_ice().map_or(0, |id| match self.card(id).kind {
            CardKind::Bran => 3,
            CardKind::Whitespace => 2,
            _ => 1,
        });
        // Vacant positions can retain the just-ended Brân encounter's mask.
        if run.broken & !7 != 0
            || (self.current_ice().is_some() && run.broken & !((1 << count) - 1) != 0)
        {
            return false;
        }
        if !matches!(
            run.server,
            Server::Hq | Server::Rd | Server::Archives | Server::Remote(_)
        ) || run.positions.len() > self.cards.len() + 1
        {
            return false;
        }
        if run
            .boosts
            .iter()
            .enumerate()
            .any(|(i, (id, _))| run.boosts[..i].iter().any(|(other, _)| other == id))
        {
            return false;
        }
        if run.boosts.iter().any(|(id, n)| {
            *n > u64::MAX - 8
                || !self
                    .cards
                    .iter()
                    .any(|c| c.id == *id && c.zone == Zone::Rig && c.kind.memory() > 0)
        }) {
            return false;
        }
        let actor = match run.stage {
            Stage::OtherPaid { priority, .. }
            | Stage::Approach { priority, .. }
            | Stage::Encounter { priority, .. } => priority,
            Stage::BranInstall { .. } | Stage::BranTrash { .. } => Player::Corp,
            _ => Player::Runner,
        };
        if !matches!(self.pending,Some(Pending::RunStep{actor:a})if a==actor) {
            return false;
        }
        match run.stage {
            Stage::Approach { .. } => self.current_ice().is_some(),
            Stage::Payment {
                effect,
                cost,
                resume,
            } => {
                let expected_cost = match effect {
                    Payment::Boost(id) => self
                        .cards
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| if c.kind == CardKind::Cleaver { 2 } else { 1 }),
                    Payment::Break(_, _) => Some(1),
                    Payment::Manegarm(_) => Some(5),
                    Payment::Trash(_) => Some(3),
                };
                if expected_cost != Some(cost) {
                    return false;
                }
                if let Payment::Trash(card) = effect {
                    return matches!(resume, PaymentWindow::Access)
                        && run.accessed.contains(&card)
                        && self.cards.iter().any(|c| {
                            c.id == card
                                && c.kind == CardKind::ManegarmSkunkworks
                                && (c.zone == Zone::Hq || c.zone == Zone::Root(run.server))
                        })
                        && self.turn().runner_credits + u32::from(run.hosted) >= 3;
                }
                if let Payment::Manegarm(source) = effect {
                    return matches!(resume, PaymentWindow::Server)
                        && self.valid_manegarm(source)
                        && self.turn().runner_credits + u32::from(run.hosted) >= 5;
                }
                let mut previous = self.clone();
                previous.run_pending(match resume {
                    PaymentWindow::Access => unreachable!(),
                    PaymentWindow::Server => Stage::ServerBoundary,
                    PaymentWindow::Encounter => Stage::Encounter {
                        priority: Player::Runner,
                        passed: false,
                    },
                    PaymentWindow::Approach => Stage::Approach {
                        priority: Player::Runner,
                        passed: false,
                    },
                    PaymentWindow::Other(point) => Stage::OtherPaid {
                        point,
                        priority: Player::Runner,
                        passed: false,
                    },
                });
                let action = match effect {
                    Payment::Manegarm(_) | Payment::Trash(_) => unreachable!(),
                    Payment::Boost(id) => Action::Boost(id),
                    Payment::Break(breaker, subroutines) => Action::Break {
                        breaker,
                        subroutines,
                    },
                };
                previous.valid_run() && previous.run_choices().contains(&action)
            }
            Stage::Encounter { .. } => self.current_ice().is_some_and(|id| self.card(id).rezzed),
            Stage::BranInstall { source } => {
                self.current_ice() == Some(source)
                    && self.card(source).kind == CardKind::Bran
                    && self.card(source).rezzed
                    && run.broken & 1 == 0
            }
            Stage::BranTrash {
                source,
                card,
                inward,
            } => {
                inward < run.positions.len()
                    && run.cursor == Some(inward + 1)
                    && run.positions[inward].is_none()
                    && (self.current_ice() == Some(source)
                        || (self.current_ice().is_none()
                            && self
                                .cards
                                .iter()
                                .any(|c| c.id == source && c.zone == Zone::Archives)))
                    && self
                        .cards
                        .iter()
                        .any(|c| c.id == source && c.kind == CardKind::Bran)
                    && self
                        .cards
                        .iter()
                        .any(|c| c.id == card && c.kind.ice() && c.zone == Zone::Installing)
            }
            Stage::OtherPaid { .. } => true,
            Stage::Movement => true,
            Stage::Breach => {
                run.cursor.is_none()
                    && run.server == Server::Hq
                    && run.hq_remaining.is_some()
                    && !self.breach_choices().is_empty()
            }
            Stage::Access { card } => {
                run.cursor.is_none()
                    && run.server == Server::Hq
                    && run.hq_remaining.is_some()
                    && run.accessed.contains(&card)
                    && self.cards.iter().any(|c| {
                        c.id == card && (c.zone == Zone::Hq || c.zone == Zone::Root(run.server))
                    })
            }
            Stage::ServerBoundary => run.cursor.is_none(),
            Stage::Manegarm { source } => self.valid_manegarm(source),
        }
    }
}
