//! Bounded diagnostic use of the real Match. Private reproduction records are
//! controller custody; the two lawful decision streams are never joined back
//! into policy input. This is neither a simulator nor a general evaluator.
use crate::{
    game::{Action, Checkpoint, EngineeringPosition, Match, Observation, Player, Server},
    policies::Policy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    time::Instant,
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const WITNESS: &str = "source-hq-witness-v1; CR26.03; pinned Gateway clauses";
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub executable: String,
}
impl Identity {
    pub fn current() -> Result<Self> {
        let mut hasher = Sha256::new();
        let mut file = File::open(std::env::current_exe()?)?;
        let mut buffer = [0; 65536];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        Ok(Self {
            executable: format!("{:x}", hasher.finalize()),
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Limits {
    pub decisions: usize,
    pub milliseconds: u64,
    pub bytes: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            decisions: 4000,
            milliseconds: 120_000,
            bytes: 256 * 1024 * 1024,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<()> {
        if self.decisions > 4000
            || self.milliseconds > 120_000
            || self.bytes > 256 * 1024 * 1024
            || self.bytes < 65536
        {
            return Err(
                "Diagnostic limits exceed the bounded contract (or cannot hold a header).".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ending {
    Terminal { winner: Player, reason: String },
    Censored(String),
    Failed(String),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RoleReport {
    pub decisions: usize,
    pub policy_nanoseconds: u64,
    pub max_policy_nanoseconds: u64,
    pub unassessed: usize,
    pub assessed: usize,
    pub mistakes: usize,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub enum Assessment {
    Unassessed,
    Checked { case: &'static str, mistake: bool },
}

/// A finite source-derived check, not a value function or prose evaluator.
/// Future outcomes and private engine state cannot enter this interface.
pub fn assess(view: &Observation, selected: Action) -> Assessment {
    let Some(d) = &view.decision else {
        return Assessment::Unassessed;
    };
    if !d.choices.contains(&selected) {
        return Assessment::Unassessed;
    }
    if d.actor == Player::Corp && view.rd_count < 2 && d.choices.contains(&Action::DrawAgendaCards)
    {
        return Assessment::Checked {
            case: "hub_deck_exhaustion",
            mistake: selected == Action::DrawAgendaCards,
        };
    }
    if d.actor == Player::Runner
        && d.choices == [Action::Continue, Action::JackOut]
        && view.hand.len() < 2
        && view.run.as_ref().is_some_and(|run| {
            run.karuna_continuation
                && run
                    .ice
                    .as_ref()
                    .is_some_and(|ice| ice.printing == Some("30047"))
                && run.subroutines.get(1).is_some_and(|s| !s.broken)
        })
    {
        return Assessment::Checked {
            case: "karuna_exposed_lethal",
            mistake: selected == Action::Continue,
        };
    }
    let points = |printing| match printing {
        Some("30067") => 2,
        Some("30069") => 3,
        Some("30070") => 1,
        _ => 0,
    };
    let wins = |action: Action| match action {
        Action::Score(id) if d.actor == Player::Corp => view
            .servers
            .iter()
            .flat_map(|s| &s.root)
            .any(|c| c.instance == Some(id) && view.agenda_points[0] + points(c.printing) >= 6),
        Action::StealAgenda if d.actor == Player::Runner => view
            .accessed
            .as_ref()
            .is_some_and(|c| view.agenda_points[1] + points(c.printing) >= 6),
        _ => false,
    };
    if d.choices.iter().copied().any(wins) {
        return Assessment::Checked {
            case: "immediate_winning_agenda",
            mistake: !wins(selected),
        };
    }
    Assessment::Unassessed
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Summary {
    pub ending: Ending,
    pub decisions: usize,
    pub elapsed_milliseconds: u64,
    pub record_bytes: u64,
    pub roles: [RoleReport; 2],
    pub legality_failures: usize,
    /// Separate paired controls supply privacy evidence; ordinary games do not.
    pub privacy: String,
    pub final_state: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "record")]
enum PrivateRow {
    Start {
        version: u32,
        identity: Identity,
        basis: Basis,
        limits: Limits,
        policies: [String; 2],
        policy_randomness: String,
        policy_work: String,
        checkpoint: Box<Checkpoint>,
    },
    Step {
        actor: Player,
        revision: u64,
        action: Action,
        state_after: String,
    },
    Refused {
        actor: Player,
        revision: u64,
        action: Action,
        reason: String,
    },
    End {
        summary: Summary,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize)]
enum Basis {
    Introductory { seed: u64 },
    SourceHqCase,
}
impl Basis {
    fn game(self) -> Result<Match> {
        Ok(match self {
            Self::Introductory { seed } => Match::introductory(seed),
            Self::SourceHqCase => Match::engineering_turns(EngineeringPosition::hq_access(42))?,
        })
    }
}

#[derive(Serialize)]
struct LawfulRow<'a> {
    observation: &'a Observation,
    selected: Action,
    policy: &'a str,
    policy_nanoseconds: u64,
    assessment: Assessment,
}

/// Diagnostic records use fresh paths, never replace a native save. Partial I/O
/// failures leave the prefix in place; absence of End means an interrupted file.
struct Recorder {
    game: Match,
    private: File,
    lawful: [File; 2],
    policies: [Policy; 2],
    limits: Limits,
    started: Instant,
    bytes: u64,
    roles: [RoleReport; 2],
    decisions: usize,
    legality_failures: usize,
}

fn create(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}
fn encoded(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn state_digest(game: &Match) -> Result<String> {
    Ok(digest(&serde_json::to_vec(&game.checkpoint())?))
}
fn check_ending(game: &Match, ending: &Ending) -> Result<()> {
    let view = game.observe(Player::Corp);
    match (view.match_winner, ending) {
        (
            Some(winner),
            Ending::Terminal {
                winner: claimed,
                reason,
            },
        ) if winner == *claimed && view.terminal_reason == Some(reason.as_str()) => Ok(()),
        (None, Ending::Censored(_) | Ending::Failed(_)) => Ok(()),
        _ => Err("Diagnostic result disagrees with the rules-defined terminal state.".into()),
    }
}

impl Recorder {
    fn new(path: &Path, seed: u64, limits: Limits, policies: [Policy; 2]) -> Result<Self> {
        if policies[0].role() != Player::Corp || policies[1].role() != Player::Runner {
            return Err("Policies must occupy their declared roles.".into());
        }
        let mut recorder = Self::with_labels(
            path,
            Basis::Introductory { seed },
            limits,
            policies.map(Policy::identity),
        )?;
        recorder.policies = policies;
        Ok(recorder)
    }
    fn with_labels(path: &Path, basis: Basis, limits: Limits, labels: [String; 2]) -> Result<Self> {
        limits.validate()?;
        let game = basis.game()?;
        let start = encoded(&PrivateRow::Start {
            version: 1,
            identity: Identity::current()?,
            basis,
            limits,
            policies: labels,
            policy_randomness: "deterministic, stateless, no random stream".into(),
            policy_work: "one selector invocation per decision; no search".into(),
            checkpoint: Box::new(game.checkpoint()),
        })?;
        if start.len() as u64 + 8192 > limits.bytes {
            return Err("Initial checkpoint exceeds record budget.".into());
        }
        fs::create_dir(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        let mut private = create(&path.join("private.jsonl"))?;
        let lawful = [
            create(&path.join("corp.jsonl"))?,
            create(&path.join("runner.jsonl"))?,
        ];
        private.write_all(&start)?;
        Ok(Self {
            game,
            private,
            lawful,
            policies: [Policy::CorpV0, Policy::RunnerV0],
            limits,
            started: Instant::now(),
            bytes: start.len() as u64,
            roles: Default::default(),
            decisions: 0,
            legality_failures: 0,
        })
    }
    pub fn boundary(&self) -> Option<Ending> {
        let view = self.game.observe(Player::Corp);
        if let Some(winner) = view.match_winner {
            return Some(Ending::Terminal {
                winner,
                reason: view
                    .terminal_reason
                    .unwrap_or("unspecified terminal")
                    .into(),
            });
        }
        if self.decisions >= self.limits.decisions {
            return Some(Ending::Censored("decision limit".into()));
        }
        if self.started.elapsed().as_millis() >= self.limits.milliseconds.into() {
            return Some(Ending::Censored("wall time limit".into()));
        }
        None
    }
    pub fn acting(&self) -> Option<Observation> {
        [Player::Corp, Player::Runner]
            .into_iter()
            .map(|p| self.game.observe(p))
            .find(|v| v.decision.is_some())
    }
    /// The explicit action entry also records independently authored source
    /// witnesses, without presenting that itinerary as an opponent policy.
    pub fn choose(
        &mut self,
        action: Action,
        policy: &str,
        nanoseconds: u64,
    ) -> Result<Option<Ending>> {
        if let Some(end) = self.boundary() {
            return Ok(Some(end));
        }
        let Some(view) = self.acting() else {
            return Ok(Some(Ending::Failed(
                "nonterminal state has no decision".into(),
            )));
        };
        let decision = view.decision.as_ref().unwrap();
        let assessment = assess(&view, action);
        let row = encoded(&LawfulRow {
            observation: &view,
            selected: action,
            policy,
            policy_nanoseconds: nanoseconds,
            assessment: assess(&view, action),
        })?;
        if self.bytes + row.len() as u64 + 8192 > self.limits.bytes {
            return Ok(Some(Ending::Censored("record byte limit".into())));
        }
        if let Err(error) = self.game.choose(decision.actor, decision.revision, action) {
            let refusal = encoded(&PrivateRow::Refused {
                actor: decision.actor,
                revision: decision.revision,
                action,
                reason: error.into(),
            })?;
            self.lawful[decision.actor.index()].write_all(&row)?;
            self.private.write_all(&refusal)?;
            self.bytes += (row.len() + refusal.len()) as u64;
            self.legality_failures += 1;
            return Ok(Some(Ending::Failed(format!(
                "illegal or refused transition: {error}"
            ))));
        }
        let step = encoded(&PrivateRow::Step {
            actor: decision.actor,
            revision: decision.revision,
            action,
            state_after: state_digest(&self.game)?,
        })?;
        self.lawful[decision.actor.index()].write_all(&row)?;
        self.private.write_all(&step)?;
        self.bytes += (row.len() + step.len()) as u64;
        self.decisions += 1;
        let report = &mut self.roles[decision.actor.index()];
        report.decisions += 1;
        match assessment {
            Assessment::Unassessed => report.unassessed += 1,
            Assessment::Checked { mistake, .. } => {
                report.assessed += 1;
                report.mistakes += usize::from(mistake);
            }
        }
        report.policy_nanoseconds += nanoseconds;
        report.max_policy_nanoseconds = report.max_policy_nanoseconds.max(nanoseconds);
        Ok(None)
    }
    pub fn finish(mut self, ending: Ending) -> Result<Summary> {
        check_ending(&self.game, &ending)?;
        let mut summary = Summary {
            ending,
            decisions: self.decisions,
            elapsed_milliseconds: self.started.elapsed().as_millis() as u64,
            record_bytes: self.bytes,
            roles: self.roles,
            legality_failures: self.legality_failures,
            privacy: "not assessed by match outcomes; see paired controls".into(),
            final_state: state_digest(&self.game)?,
        };
        // Include the final record itself in the reported byte count.
        for _ in 0..3 {
            summary.record_bytes = self.bytes
                + encoded(&PrivateRow::End {
                    summary: summary.clone(),
                })?
                .len() as u64;
        }
        let end = encoded(&PrivateRow::End {
            summary: summary.clone(),
        })?;
        self.private.write_all(&end)?;
        self.private.sync_all()?;
        for file in &self.lawful {
            file.sync_all()?;
        }
        Ok(summary)
    }
    pub fn run(mut self) -> Result<Summary> {
        loop {
            if let Some(end) = self.boundary() {
                return self.finish(end);
            }
            let Some(view) = self.acting() else {
                return self.finish(Ending::Failed("nonterminal state has no decision".into()));
            };
            let policy = self.policies[view.decision.as_ref().unwrap().actor.index()];
            let start = Instant::now();
            let selected = policy.choose(&view);
            let elapsed = start.elapsed().as_nanos() as u64;
            let Some(action) = selected else {
                return self.finish(Ending::Failed("policy returned no match".into()));
            };
            if let Some(end) = self.choose(action, &policy.identity(), elapsed)? {
                return self.finish(end);
            }
        }
    }
}

pub fn record_introductory(seed: u64, path: &Path, limits: Limits) -> Result<Summary> {
    Recorder::new(path, seed, limits, [Policy::CorpV0, Policy::RunnerV0])?.run()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Split {
    Development,
    HeldOut,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairBlock {
    pub role: Player,
    pub seed: u64,
    pub split: Split,
    pub incumbent: Policy,
    pub candidate: Policy,
    pub opponent: Policy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComparisonPlan {
    pub blocks: Vec<PairBlock>,
    /// Custody for later role candidates; these seeds are prohibited in this run.
    pub reserved_seeds: Vec<u64>,
    pub limits: Limits,
}
impl ComparisonPlan {
    pub fn pilot() -> Self {
        Self {
            blocks: [
                (Player::Corp, 3, Split::Development),
                (Player::Corp, 7039, Split::HeldOut),
                (Player::Runner, 89, Split::Development),
                (Player::Runner, 9151, Split::HeldOut),
            ]
            .into_iter()
            .map(|(role, seed, split)| {
                let (policy, opponent) = match role {
                    Player::Corp => (Policy::CorpV0, Policy::RunnerV0),
                    Player::Runner => (Policy::RunnerV0, Policy::CorpV0),
                };
                PairBlock {
                    role,
                    seed,
                    split,
                    incumbent: policy,
                    candidate: policy,
                    opponent,
                }
            })
            .collect(),
            reserved_seeds: vec![104729, 130363, 155921, 196613],
            limits: Limits::default(),
        }
    }
    fn validate(&self) -> Result<()> {
        self.limits.validate()?;
        if self.blocks.is_empty() || self.blocks.len() > 4 {
            return Err(
                "A comparison permits one to four paired blocks (at most eight games).".into(),
            );
        }
        for block in &self.blocks {
            if block.incumbent.role() != block.role
                || block.candidate.role() != block.role
                || block.opponent.role() == block.role
            {
                return Err("Comparison policies have incorrect role assignments.".into());
            }
            if self.reserved_seeds.contains(&block.seed)
                || (block.split == Split::HeldOut && [3, 42, 89, 117].contains(&block.seed))
            {
                return Err("Held-out/development seed custody conflicts with this block.".into());
            }
            if self
                .blocks
                .iter()
                .any(|b| b.seed == block.seed && b.split != block.split)
            {
                return Err("One seed cannot occupy both splits.".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameReport {
    pub block: usize,
    pub arm: String,
    pub summary: Option<Summary>,
    /// Includes a failed output operation or a declared limit before this game.
    pub unavailable: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub games: Vec<GameReport>,
    pub elapsed_milliseconds: u64,
    pub record_bytes: u64,
    pub decisions_per_second: f64,
    pub games_per_second: f64,
}

/// One role changes per paired block. Equal setup seeds do not promise equal
/// later luck when policies take different paths. Every planned arm is retained.
pub fn run_comparison(plan: &ComparisonPlan, path: &Path) -> Result<ComparisonReport> {
    plan.validate()?;
    fs::create_dir(path)?;
    let started = Instant::now();
    let declaration = encoded(plan)?;
    create(&path.join("plan.json"))?.write_all(&declaration)?;
    let mut bytes = declaration.len() as u64;
    let mut games = Vec::new();
    let mut failure = None;
    for (block_index, block) in plan.blocks.iter().enumerate() {
        for (arm, policy) in [
            ("incumbent", block.incumbent),
            ("candidate", block.candidate),
        ] {
            let left_ms = plan
                .limits
                .milliseconds
                .saturating_sub(started.elapsed().as_millis() as u64);
            let left_bytes = plan.limits.bytes.saturating_sub(bytes + 65536);
            let unavailable = failure.clone().or_else(|| {
                if left_ms == 0 {
                    Some("batch wall time limit".into())
                } else if left_bytes < 65536 {
                    Some("batch record byte limit".into())
                } else {
                    None
                }
            });
            let mut result = GameReport {
                block: block_index,
                arm: arm.into(),
                summary: None,
                unavailable,
            };
            if result.unavailable.is_none() {
                let policies = match block.role {
                    Player::Corp => [policy, block.opponent],
                    Player::Runner => [block.opponent, policy],
                };
                let limits = Limits {
                    decisions: plan.limits.decisions,
                    milliseconds: left_ms,
                    bytes: left_bytes,
                };
                let directory = path.join(format!("{block_index}-{arm}"));
                match Recorder::new(&directory, block.seed, limits, policies)
                    .and_then(Recorder::run)
                {
                    Ok(summary) => {
                        bytes += summary.record_bytes;
                        result.summary = Some(summary);
                    }
                    Err(error) => {
                        // Preserve partial output and keep all remaining arms visible.
                        bytes += ["private.jsonl", "corp.jsonl", "runner.jsonl"]
                            .iter()
                            .filter_map(|name| fs::metadata(directory.join(name)).ok())
                            .map(|m| m.len())
                            .sum::<u64>();
                        failure = Some(format!("recording failed: {error}"));
                        result.unavailable = failure.clone();
                    }
                }
            }
            games.push(result);
        }
    }
    let seconds = started.elapsed().as_secs_f64();
    let decisions: usize = games
        .iter()
        .filter_map(|g| g.summary.as_ref())
        .map(|s| s.decisions)
        .sum();
    let completed = games
        .iter()
        .filter(|g| {
            g.summary
                .as_ref()
                .is_some_and(|s| matches!(s.ending, Ending::Terminal { .. }))
        })
        .count();
    let mut report = ComparisonReport {
        games,
        elapsed_milliseconds: started.elapsed().as_millis() as u64,
        record_bytes: bytes,
        decisions_per_second: decisions as f64 / seconds.max(f64::EPSILON),
        games_per_second: completed as f64 / seconds.max(f64::EPSILON),
    };
    for _ in 0..3 {
        report.record_bytes = bytes + encoded(&report)?.len() as u64;
    }
    create(&path.join("report.json"))?.write_all(&encoded(&report)?)?;
    Ok(report)
}

/// Deliberately selected hostile case from docs/research/evidence.md: Overclock,
/// Brân's inward install/trash, normal Palisade rez, hosted break payments and
/// Whitespace's pool loss, suspended at Manegarm. This is an itinerary, not a bot.
pub fn record_source_case(path: &Path) -> Result<(Summary, Observation)> {
    let mut recorder = Recorder::with_labels(
        path,
        Basis::SourceHqCase,
        Limits {
            decisions: 128,
            ..Limits::default()
        },
        [WITNESS.into(), WITNESS.into()],
    )?;
    loop {
        if let Some(end) = recorder.boundary() {
            return Err(format!("Source itinerary did not reach its suspension: {end:?}").into());
        }
        let view = recorder
            .acting()
            .ok_or("Source itinerary lost its decision")?;
        let choices = &view.decision.as_ref().unwrap().choices;
        if choices.contains(&Action::PayManegarmCredits) {
            let end = Ending::Censored("selected source-case suspension at Manegarm".into());
            return Ok((recorder.finish(end)?, view));
        }
        let action = source_choice(&view).ok_or("Source itinerary has no supported action")?;
        if let Some(end) = recorder.choose(action, WITNESS, 0)? {
            return Err(format!("Source itinerary stopped: {end:?}").into());
        }
    }
}

fn source_choice(view: &Observation) -> Option<Action> {
    let choices = &view.decision.as_ref()?.choices;
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::PlayCard(id) if view.hand.iter().any(|c| c.instance == Some(*id) && c.printing == Some("30029")))) { return Some(*action); }
    if choices.contains(&Action::RunServer(Server::Hq)) {
        return Some(Action::RunServer(Server::Hq));
    }
    if let Some(action) = choices
        .iter()
        .find(|a| matches!(a, Action::InstallInward(_) | Action::Rez(_)))
    {
        return Some(*action);
    }
    if let Some(action) = choices.iter().find(|a| matches!(a, Action::TrashInstalled(id) if view.servers.iter().flat_map(|s| &s.ice).any(|c| c.instance == Some(*id) && c.printing == Some("30039")))) { return Some(*action); }
    for action in [Action::FinishInstall, Action::PayRunCredits(1)] {
        if choices.contains(&action) {
            return Some(action);
        }
    }
    if let Some(run) = &view.run {
        let mask = match run.ice.as_ref().and_then(|ice| ice.printing) {
            Some("30072") => 1,
            Some("30074") => 2,
            _ => 0,
        };
        if let Some(action) = choices
            .iter()
            .find(|a| matches!(a, Action::Break { subroutines, .. } if *subroutines == mask))
        {
            return Some(*action);
        }
    }
    [Action::Continue, Action::PassPriority]
        .into_iter()
        .find(|a| choices.contains(a))
}

/// Refuse a different executable, corrupt step, or unfinished record. The
/// source-to-build association lives in the retained exact-source build handoff;
/// a runtime Git label would not prove which source produced this executable.
/// Only checkpoints are decoded here; lawful observations are never replay input.
pub fn reproduce(path: &Path) -> Result<String> {
    let bytes = unfolded_record_replacement::read(
        path,
        32 * 1024 * 1024,
        |s| -> Box<dyn std::error::Error> { s.into() },
    )?;
    let text = std::str::from_utf8(&bytes)?;
    let mut rows = text.lines();
    let Some(first) = rows.next() else {
        return Err("Empty reproduction record.".into());
    };
    let PrivateRow::Start {
        version: 1,
        identity,
        basis,
        checkpoint,
        policies,
        limits,
        ..
    } = serde_json::from_str(first)?
    else {
        return Err("Unsupported reproduction record.".into());
    };
    if identity != Identity::current()? {
        return Err("Reproduction requires the exact executable identity.".into());
    }
    limits.validate()?;
    let supported = match basis {
        Basis::Introductory { .. } => {
            [
                Policy::CorpV0.identity(),
                Policy::CorpV1.identity(),
                Policy::CorpV2.identity(),
            ]
            .contains(&policies[0])
                && [Policy::RunnerV0.identity(), Policy::RunnerV1.identity()].contains(&policies[1])
        }
        Basis::SourceHqCase => policies == [WITNESS.to_string(), WITNESS.to_string()],
    };
    if !supported {
        return Err("Unsupported or changed policy identities.".into());
    }
    let mut game = Match::resume(*checkpoint)?;
    if state_digest(&game)? != state_digest(&basis.game()?)? {
        return Err("Initial state differs from the declared seed/content case.".into());
    }
    let mut end = None;
    let mut decisions = 0;
    let mut refusals = 0;
    for line in rows {
        if end.is_some() {
            return Err("Data after final record.".into());
        }
        match serde_json::from_str(line)? {
            PrivateRow::Step {
                actor,
                revision,
                action,
                state_after,
            } => {
                game.choose(actor, revision, action)?;
                if state_digest(&game)? != state_after {
                    return Err("Reproduced state differs at a recorded decision.".into());
                }
                decisions += 1;
            }
            PrivateRow::Refused {
                actor,
                revision,
                action,
                reason,
            } => {
                let before = state_digest(&game)?;
                if game.choose(actor, revision, action).err() != Some(reason.as_str())
                    || state_digest(&game)? != before
                {
                    return Err("Recorded refusal does not reproduce without mutation.".into());
                }
                refusals += 1;
            }
            PrivateRow::End { summary } => {
                if state_digest(&game)? != summary.final_state {
                    return Err("Reproduced final state differs.".into());
                }
                check_ending(&game, &summary.ending)?;
                if decisions != summary.decisions {
                    return Err("Reproduced decision count differs.".into());
                }
                if refusals != summary.legality_failures {
                    return Err("Reproduced refusal count differs.".into());
                }
                end = Some(summary.final_state);
            }
            PrivateRow::Start { .. } => return Err("Duplicate start record.".into()),
        }
    }
    end.ok_or_else(|| "Interrupted reproduction record: no final result.".into())
}
