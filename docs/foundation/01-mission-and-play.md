# Mission and play contract

## Authority and status

Jon owns product acceptance. This foundation records his interview decisions through **2026-11-07** and the technical recommendations derived from the accompanying research. It specializes [the constitution](../principles/principles-for-repos.md), particularly `PURP`, `AUTH`, `EVID`, and `GOV`; it does not replace it. Technical designs below are implementation starting points, not evidence of fitness or authorization for every possible subsystem.

Purpose: a personal recreational application that lets Jon enjoy complete Netrunner games alone, with the feel of a physical table and a credible opponent. Success is actual enjoyable play, including returning to an unfinished game without reconstructing setup from memory.

The only named products are `arkham-unfolded` and `netrunner-unfolded`; `unfolded-tabletop` contains the justified common mechanisms extracted for their first coordinated slice. On **2026-11-07**, Jon judged The Gathering complete enough to proceed with Netrunner and requested preparation for its first GitHub specification. That decision cleared the previous Gathering prerequisite and selected Netrunner preparation ahead of further Arkham feature work; it did not declare either product complete or discharge Arkham's own acceptance obligations. Arkham remains the first place to investigate existing mechanisms, including tooling, and its behavior must survive coordinated extraction.

Jon selected a bounded playable run position within the eventual match path. [GitHub issue #1](https://github.com/joeloverbeck/netrunner-unfolded/issues/1) records its specification, later implementation, acceptance and publication; [shared code and development](05-shared-code-and-development.md#first-specification-boundary) preserves the accepted design input and initial appetite. The implemented entry is described in [Brân at HQ](../bran-run.md); later work requires its own bounded authorization. The prerequisite and asset/reuse decisions apply `PURP-1`, `WORK-1`, `AUTH-2`, `AGENT-5`, and `ADOPT-3` without declaring the full milestone complete.

## First complete playable milestone

Both official System Gateway teaching configurations are supported: introductory decks, then those decks with their ten-card boosters. [Rules and content](02-rules-and-content.md) fixes their identities, counts, and differing victory thresholds. Jon reaffirmed on 2026-11-07 that initial full-match selection stays within these teaching identities and configurations; broader identity/deck selection comes later. Both human-as-Runner and human-as-Corp must work through full games against bots. Every legal choice and interaction reachable with these decks is in scope, including unfavorable, unusual, and game-ending branches. There is no manual rules-repair mode that makes unfinished rules count as complete.

The experience includes:

- A 3D tabletop with cards and tokens, the human seated near the camera, and readable inspection panels.
- Clicking cards and selecting actions; the application moves pieces and performs bookkeeping.
- Save/resume, safe undo before new information or an opponent response, an explanatory action history, and contextual explanations of rules and available actions.
- Bots for both roles targeting experienced casual play: sensible plans, credible bluffing, and few obvious mistakes. This target is not yet demonstrated.

Platform is Jon's Ubuntu PC; language is English. Gameplay is entirely offline after asset preparation, without an LLM. All project-owned executable code is Rust, including asset preparation, tests, experiments, and training tools. Existing external tools and libraries may have other implementation languages. Original code uses MIT; public source does not promise a support service or other-platform compatibility.

Difficult bot decisions may use up to 15–20 seconds when that improves play; routine decisions should be much quicker. Development and learning use Jon's PC only, with occasional bounded overnight runs. There is no cloud-training budget.

## Acceptance and limits

The full milestone needs rules coverage for the exact decks, human play in both roles/configurations, and the contextual support above. Inspect source-derived difficult interactions separately from ordinary full games. Exercise saving during a run or unresolved choice, resuming after restart, undo barriers, and offline asset discovery. Human acceptance assesses enjoyment and usability; bot evaluation supplies separate evidence about playing strength. A win against a weak baseline or a polished board does not substitute for either.

Do not advertise literal proof of all possible game histories. The practical claim requires no known unsupported legal interaction in this scope, targeted interaction coverage, varied complete-game evidence, and reproducible diagnosis of failures. A blocked branch remains a deficit.

Not part of this milestone: arbitrary deckbuilding/import, the rest of System Gateway, later sets, multiplayer/network services, VR, other operating systems, guided tutorial, user-facing completed-game replay, or crates.io publication. Future card pools remain a design concern, not permission to implement them now. Diagnostic reproduction is distinct from a replay product.

Current bottlenecks for the full milestone are Netrunner timing/hidden-information fidelity and opponent quality within the local budget. [Shared code and development](05-shared-code-and-development.md#first-specification-boundary) records the accepted first increment's design. GitHub Issues and PRs are the sole active work-control plane; these documents are not tickets or a milestone completion ledger.
