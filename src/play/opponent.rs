//! A decision request owns only the lawful observation. The controller alone
//! owns the game/save and admits a finished proposal against its current epoch.
use super::*;
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

/// Leave one second within the product's twenty-second ceiling for polling and
/// the ordinary validated save commit. Routine selectors do no search.
pub const OPPONENT_BUDGET: Duration = Duration::from_secs(19);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum OpponentOutcome {
    Selected,
    TimeoutFallback,
    UnavailableFallback,
    Cancelled,
    Stale,
    SaveFailed,
}

/// Controller diagnostics. The native seat publishes a redacted subset: the
/// count of legal alternatives belongs to the opponent's private observation.
#[derive(Clone, Debug, Serialize)]
pub struct OpponentReport {
    pub policy: String,
    pub revision: u64,
    pub elapsed_nanoseconds: u64,
    pub legal_alternatives: usize,
    pub selector_invocations: Option<u8>,
    pub outcome: OpponentOutcome,
    pub committed: bool,
}

#[derive(Clone)]
pub struct OpponentRequest {
    view: Observation,
    policy: Policy,
    epoch: u64,
    cancellation: Arc<AtomicBool>,
    started: Instant,
    budget: Duration,
}

#[derive(Clone)]
pub struct OpponentProposal {
    actor: Player,
    revision: u64,
    policy: Policy,
    epoch: u64,
    action: Option<Action>,
    cancellation: Arc<AtomicBool>,
    report: OpponentReport,
}
impl OpponentProposal {
    pub fn report(&self) -> &OpponentReport {
        &self.report
    }
}
impl OpponentRequest {
    fn proposal(
        &self,
        action: Option<Action>,
        outcome: OpponentOutcome,
        invocations: u8,
    ) -> OpponentProposal {
        let decision = self
            .view
            .decision
            .as_ref()
            .expect("request has an acting decision");
        OpponentProposal {
            actor: decision.actor,
            revision: decision.revision,
            policy: self.policy,
            epoch: self.epoch,
            action,
            cancellation: self.cancellation.clone(),
            report: OpponentReport {
                policy: self.policy.identity(),
                revision: decision.revision,
                elapsed_nanoseconds: self.started.elapsed().as_nanos().min(u128::from(u64::MAX))
                    as u64,
                legal_alternatives: decision.choices.len(),
                selector_invocations: Some(invocations),
                outcome,
                committed: false,
            },
        }
    }
    fn fallback(&self, outcome: OpponentOutcome, invocations: u8) -> OpponentProposal {
        let choices = &self.view.decision.as_ref().unwrap().choices;
        // Emergency continuity only, never the normal Corp strategy.
        let action = [
            Action::PassPriority,
            Action::DeclineInstall,
            Action::DeclineAgendaAbility,
            Action::GainCredit,
        ]
        .into_iter()
        .find(|a| choices.contains(a))
        .or_else(|| choices.first().copied());
        self.proposal(action, outcome, invocations)
    }
    pub fn compute(self) -> OpponentProposal {
        if self.cancellation.load(Ordering::Relaxed) {
            return self.proposal(None, OpponentOutcome::Cancelled, 0);
        }
        if self.started.elapsed() >= self.budget {
            return self.fallback(OpponentOutcome::TimeoutFallback, 0);
        }
        // Finite deterministic selector over the admitted deck; no world clones,
        // random stream, mutable memory, external calls or unbounded search.
        let action = self.policy.choose(&self.view);
        if self.cancellation.load(Ordering::Relaxed) {
            return self.proposal(None, OpponentOutcome::Cancelled, 1);
        }
        if self.started.elapsed() >= self.budget {
            return self.fallback(OpponentOutcome::TimeoutFallback, 1);
        }
        match action {
            Some(action)
                if self
                    .view
                    .decision
                    .as_ref()
                    .unwrap()
                    .choices
                    .contains(&action) =>
            {
                self.proposal(Some(action), OpponentOutcome::Selected, 1)
            }
            _ => self.fallback(OpponentOutcome::UnavailableFallback, 1),
        }
    }
}

pub(super) struct Worker {
    request: OpponentRequest,
    receiver: Mutex<mpsc::Receiver<OpponentProposal>>,
}

impl Play {
    pub fn opponent_policy(&self) -> Policy {
        self.policy
    }
    pub fn opponent_report(&self) -> Option<&OpponentReport> {
        self.opponent_report.as_ref()
    }
    pub fn prepare_opponent(&self, budget: Duration) -> Result<Option<OpponentRequest>, String> {
        if self.admission != Admission::Ready {
            return Err("Artwork or save is not admitted.".into());
        }
        if let Some(error) = &self.persistence_failure {
            return Err(error.clone());
        }
        let Some(view) = self
            .observe(self.human.other())
            .filter(|v| v.decision.is_some())
        else {
            return Ok(None);
        };
        Ok(Some(OpponentRequest {
            view,
            policy: self.policy,
            epoch: self.opponent_epoch,
            cancellation: self.opponent_cancellation.clone(),
            started: Instant::now(),
            budget: budget.min(OPPONENT_BUDGET),
        }))
    }
    pub fn cancel_opponent(&mut self) {
        if let Some(worker) = self.opponent_worker.take() {
            let mut report = worker
                .request
                .proposal(None, OpponentOutcome::Cancelled, 0)
                .report;
            // Discarded work may already be running; its completed count is unknown.
            report.selector_invocations = None;
            self.opponent_report = Some(report);
        }
        self.opponent_cancellation.store(true, Ordering::Relaxed);
        self.opponent_cancellation = Arc::new(AtomicBool::new(false));
        self.opponent_epoch = self.opponent_epoch.wrapping_add(1);
    }
    pub fn commit_opponent(&mut self, mut proposal: OpponentProposal) -> Result<(), String> {
        if !Arc::ptr_eq(&proposal.cancellation, &self.opponent_cancellation)
            || proposal.epoch != self.opponent_epoch
            || proposal.policy != self.policy
            || proposal.actor != self.human.other()
            || self.revision() != Some(proposal.revision)
            || proposal.action.is_none()
        {
            if proposal.report.outcome != OpponentOutcome::Cancelled {
                proposal.report.outcome = OpponentOutcome::Stale;
            }
            self.opponent_report = Some(proposal.report);
            return Err(
                "Opponent proposal is stale or cancelled; the committed game is unchanged.".into(),
            );
        }
        let mut candidate = self.candidate()?;
        candidate.choose(proposal.actor, proposal.revision, proposal.action.unwrap())?;
        let result = self.commit(candidate);
        proposal.report.committed = result.is_ok();
        if result.is_err() {
            proposal.report.outcome = OpponentOutcome::SaveFailed;
        }
        self.opponent_report = Some(proposal.report);
        result
    }
    /// Poll on every render frame, even while inspection blocks new decisions.
    /// Computation owns no controller, filesystem handle or actual hidden world.
    pub fn poll_opponent(&mut self, allow_start: bool) -> Result<(), String> {
        if let Some(worker) = self.opponent_worker.take() {
            let received = worker
                .receiver
                .lock()
                .map_err(|_| "Opponent result channel is poisoned")?
                .try_recv();
            let proposal = match received {
                Ok(proposal) => proposal,
                Err(mpsc::TryRecvError::Empty)
                    if worker.request.started.elapsed() < worker.request.budget =>
                {
                    self.opponent_worker = Some(worker);
                    return Ok(());
                }
                Err(error) => {
                    worker.request.cancellation.store(true, Ordering::Relaxed);
                    let outcome = if matches!(error, mpsc::TryRecvError::Empty) {
                        OpponentOutcome::TimeoutFallback
                    } else {
                        OpponentOutcome::UnavailableFallback
                    };
                    let mut proposal = worker.request.fallback(outcome, 0);
                    proposal.report.selector_invocations = None;
                    proposal
                }
            };
            if proposal.report.outcome != OpponentOutcome::Selected {
                eprintln!(
                    "Opponent decision status: {:?}, revision {}",
                    proposal.report.outcome, proposal.revision
                );
            }
            return self.commit_opponent(proposal);
        }
        if allow_start && let Some(request) = self.prepare_opponent(OPPONENT_BUDGET)? {
            let (sender, receiver) = mpsc::channel();
            let computation = request.clone();
            match std::thread::Builder::new()
                .name("netrunner-opponent".into())
                .spawn(move || {
                    let _ = sender.send(computation.compute());
                }) {
                Ok(_) => {
                    self.opponent_worker = Some(Worker {
                        request,
                        receiver: Mutex::new(receiver),
                    })
                }
                Err(_) => {
                    self.commit_opponent(request.fallback(OpponentOutcome::UnavailableFallback, 0))?
                }
            }
        }
        Ok(())
    }
}

impl Drop for Play {
    fn drop(&mut self) {
        self.opponent_cancellation.store(true, Ordering::Relaxed);
    }
}
