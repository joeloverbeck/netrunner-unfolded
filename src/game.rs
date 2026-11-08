//! Bounded Brân/Palisade position from issue #1, using CR 26.03.
//! This is the first match implementation, not a complete-game rules claim.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    Corp,
    Runner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    DeclineInstall,
    InstallPalisade,
    RetainBran,
    TrashBran,
    Continue,
    RezPalisade,
    DeclineRez,
    JackOut,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::DeclineInstall => "Decline installation",
            Self::InstallPalisade => "Install Palisade · free",
            Self::RetainBran => "Keep Brân",
            Self::TrashBran => "Trash Brân",
            Self::Continue => "Continue run",
            Self::RezPalisade => "Rez Palisade · 3 credits",
            Self::DeclineRez => "Leave unrezzed",
            Self::JackOut => "Jack out",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub actor: Player,
    pub revision: u64,
    pub choices: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunResult {
    EndedByIce,
    SuccessfulEmptyHq,
    JackedOut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardView {
    pub title: Option<String>,
    pub rezzed: bool,
    pub printing: Option<&'static str>,
    pub rules_text: Option<&'static str>,
}

const BRAN_TEXT: &str = "Lose 1 click: Break 1 subroutine on this ice. Only the Runner can use this ability.\nSubroutine: You may install 1 piece of ice from HQ or Archives directly inward from this ice, ignoring all costs.\nSubroutine: End the run.\nSubroutine: End the run.";
const PALISADE_TEXT: &str = "While this ice is protecting a remote server, it gets +2 strength.\nSubroutine: End the run.\nRez: 3 credits. Strength at HQ: 2.";

impl CardView {
    fn face(
        title: &str,
        printing: &'static str,
        text: &'static str,
        visible: bool,
        rezzed: bool,
    ) -> Self {
        Self {
            title: visible.then(|| title.into()),
            printing: visible.then_some(printing),
            rules_text: visible.then_some(text),
            rezzed,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub corp_identity: CardView,
    pub runner_identity: CardView,
    pub decision: Option<Decision>,
    pub corp_credits: u8,
    pub run_result: Option<RunResult>,
    pub match_winner: Option<Player>,
    pub hq_count: usize,
    /// Innermost first, independent of the current run position.
    pub ice: Vec<CardView>,
    pub archives: Vec<CardView>,
    pub approached_ice: Option<usize>,
    pub hand: Vec<CardView>,
    pub installing: Option<CardView>,
    pub history: Vec<String>,
    pub phase: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Install,
    Trash,
    MoveToReplacement,
    Rez,
    MoveToServer,
    Ended,
}

pub struct Match {
    corp_credits: u8,
    revision: u64,
    run_result: Option<RunResult>,
    step: Step,
    palisade_installed: bool,
    bran_trashed: bool,
    palisade_rezzed: bool,
    history: Vec<[String; 2]>,
    replacement_face: (&'static str, &'static str, &'static str),
}

impl Match {
    pub(crate) fn restart_position(&mut self) {
        let revision = self.revision + 1;
        *self = Self::prepared_bran_run(3);
        self.revision = revision;
    }
    pub fn prepared_bran_run(corp_credits: u8) -> Self {
        Self { corp_credits, revision: 0, run_result: None, step: Step::Install, palisade_installed: false,
            bran_trashed: false, palisade_rezzed: false,
            replacement_face: ("Palisade", "30072", PALISADE_TEXT),
            history: vec![["Brân's first subroutine resolves. The Runner has no clicks or applicable paid abilities.".into(), "Brân's first subroutine resolves. The Runner has no clicks or applicable paid abilities.".into()]] }
    }
    pub fn observe(&self, viewer: Player) -> Observation {
        let mut ice = Vec::new();
        if self.palisade_installed {
            ice.push(CardView::face(
                self.replacement_face.0,
                self.replacement_face.1,
                self.replacement_face.2,
                viewer == Player::Corp || self.palisade_rezzed,
                self.palisade_rezzed,
            ));
        }
        let bran = CardView::face("Brân 1.0", "30039", BRAN_TEXT, true, !self.bran_trashed);
        if !self.bran_trashed {
            ice.push(bran.clone());
        }
        let actor = if matches!(self.step, Step::MoveToReplacement | Step::MoveToServer) {
            Player::Runner
        } else {
            Player::Corp
        };
        Observation {
            corp_identity: CardView::face(
                "The Syndicate",
                "30077",
                "Starter game only.",
                true,
                false,
            ),
            runner_identity: CardView::face(
                "The Catalyst",
                "30076",
                "Starter game only.",
                true,
                false,
            ),
            decision: (self.run_result.is_none() && viewer == actor).then(|| Decision {
                actor,
                revision: self.revision,
                choices: match self.step {
                    Step::Install => vec![Action::DeclineInstall, Action::InstallPalisade],
                    Step::Trash => vec![Action::RetainBran, Action::TrashBran],
                    Step::MoveToReplacement | Step::MoveToServer => {
                        vec![Action::Continue, Action::JackOut]
                    }
                    Step::Rez if self.corp_credits >= 3 => {
                        vec![Action::RezPalisade, Action::DeclineRez]
                    }
                    Step::Rez => vec![Action::DeclineRez],
                    Step::Ended => vec![],
                },
            }),
            corp_credits: self.corp_credits,
            run_result: self.run_result,
            match_winner: None,
            hq_count: usize::from(
                self.step == Step::Install
                    || (!self.palisade_installed && self.step == Step::Ended),
            ),
            ice,
            archives: if self.bran_trashed {
                vec![bran]
            } else {
                vec![]
            },
            approached_ice: (self.step == Step::Rez).then_some(0),
            hand: if self.step == Step::Install
                || (!self.palisade_installed && self.step == Step::Ended)
            {
                vec![CardView::face(
                    self.replacement_face.0,
                    self.replacement_face.1,
                    self.replacement_face.2,
                    viewer == Player::Corp,
                    false,
                )]
            } else {
                vec![]
            },
            installing: (self.step == Step::Trash).then(|| {
                CardView::face(
                    self.replacement_face.0,
                    self.replacement_face.1,
                    self.replacement_face.2,
                    viewer == Player::Corp,
                    false,
                )
            }),
            history: self
                .history
                .iter()
                .map(|event| event[usize::from(viewer == Player::Runner)].clone())
                .collect(),
            phase: match self.step {
                Step::Install => "Brân: optional installation",
                Step::Trash => "Installing inward: keep or trash Brân",
                Step::MoveToReplacement => "Encounter ended · Runner may jack out",
                Step::Rez => "Approaching the new ICE · Corp may rez",
                Step::MoveToServer => "Unrezzed ICE passed · Runner may jack out",
                Step::Ended => "Run complete · match unfinished",
            },
        }
    }
    pub fn choose(
        &mut self,
        actor: Player,
        revision: u64,
        action: Action,
    ) -> Result<(), &'static str> {
        let decision = self
            .observe(actor)
            .decision
            .ok_or("No decision for this player.")?;
        if revision != decision.revision || !decision.choices.contains(&action) {
            return Err("This choice is stale or unavailable.");
        }
        let (corp, runner) = match action {
            Action::DeclineInstall => (
                "Corp declines installation. Brân's next unbroken subroutine ends the run.",
                None,
            ),
            Action::InstallPalisade => (
                "Palisade enters the play area for installation inward from Brân. Installation costs are ignored.",
                Some(
                    "Corp starts installing a facedown card inward from Brân. Installation costs are ignored.",
                ),
            ),
            Action::RetainBran => (
                "Palisade is installed inward. Brân remains; its next unbroken subroutine ends the run.",
                Some(
                    "A facedown card is installed inward. Brân remains; its next unbroken subroutine ends the run.",
                ),
            ),
            Action::TrashBran => (
                "Brân is trashed faceup to Archives. Palisade's installation completes. Brân's encounter ends; its remaining subroutines do not resolve.",
                Some(
                    "Brân is trashed faceup to Archives. The installation completes. Brân's encounter ends; its remaining subroutines do not resolve.",
                ),
            ),
            Action::Continue if self.step == Step::MoveToReplacement => (
                "The Runner continues and approaches Palisade in the new inward position.",
                Some(
                    "The Runner continues and approaches the facedown card in the new inward position.",
                ),
            ),
            Action::Continue => (
                "The Runner continues, successfully breaches empty HQ, accesses no cards, and ends the run. The match has not ended.",
                None,
            ),
            Action::RezPalisade => (
                "Corp pays 3 credits to rez Palisade. Its unbroken subroutine ends the run.",
                None,
            ),
            Action::DeclineRez => (
                "Corp declines to rez. The Runner passes the unrezzed ICE and may jack out before approaching HQ.",
                None,
            ),
            Action::JackOut => (
                "The Runner jacks out. The run ends unsuccessfully; the match has not ended.",
                None,
            ),
        };
        self.history
            .push([corp.into(), runner.unwrap_or(corp).into()]);
        match action {
            Action::InstallPalisade => self.step = Step::Trash,
            Action::DeclineInstall | Action::RetainBran => {
                self.palisade_installed = action == Action::RetainBran;
                self.run_result = Some(RunResult::EndedByIce);
                self.step = Step::Ended;
            }
            Action::TrashBran => {
                self.bran_trashed = true;
                self.palisade_installed = true;
                self.step = Step::MoveToReplacement;
            }
            Action::Continue => {
                if self.step == Step::MoveToServer {
                    self.run_result = Some(RunResult::SuccessfulEmptyHq);
                    self.step = Step::Ended;
                } else {
                    self.step = Step::Rez;
                }
            }
            Action::DeclineRez => self.step = Step::MoveToServer,
            Action::JackOut => {
                self.run_result = Some(RunResult::JackedOut);
                self.step = Step::Ended;
            }
            Action::RezPalisade => {
                self.corp_credits -= 3;
                self.palisade_rezzed = true;
                self.run_result = Some(RunResult::EndedByIce);
                self.step = Step::Ended;
            }
        }
        self.revision += 1;
        Ok(())
    }
}

/// The temporary opponent knows only its projection and submits a normal choice.
pub fn fixed_runner_response(observation: &Observation) -> Option<Action> {
    let decision = observation.decision.as_ref()?;
    (decision.actor == Player::Runner && decision.choices.contains(&Action::Continue))
        .then_some(Action::Continue)
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn changing_an_unobserved_face_does_not_change_runner_inputs() {
        let mut actual = Match::prepared_bran_run(3);
        let mut alternate = Match::prepared_bran_run(3);
        // Projection-only fixture: it admits no additional card effects.
        alternate.replacement_face = ("Unrevealed fixture", "fixture", "Private fixture text");
        assert_ne!(
            actual.observe(Player::Corp),
            alternate.observe(Player::Corp)
        );
        assert_eq!(
            actual.observe(Player::Runner),
            alternate.observe(Player::Runner)
        );
        assert_eq!(
            fixed_runner_response(&actual.observe(Player::Runner)),
            fixed_runner_response(&alternate.observe(Player::Runner))
        );
        for (actor, action) in [
            (Player::Corp, Action::InstallPalisade),
            (Player::Corp, Action::TrashBran),
            (Player::Runner, Action::Continue),
        ] {
            for game in [&mut actual, &mut alternate] {
                let revision = game.observe(actor).decision.unwrap().revision;
                game.choose(actor, revision, action).unwrap();
            }
            assert_ne!(
                actual.observe(Player::Corp),
                alternate.observe(Player::Corp)
            );
            assert_eq!(
                actual.observe(Player::Runner),
                alternate.observe(Player::Runner)
            );
        }
    }
}
