# Research evidence and alternatives

Research date: **2026-11-05**. This file is reference-only evidence for the five foundation documents. It does not create tasks, mandatory dependencies, additional product scope, or a second authority for requirements.

## What was established

The investigation inspected official rules/teaching pages, the supplied PDF, official card data and 25 relevant ruling files, selected Arkham source modules and governing documents, Chiriboga's bot code, and primary imperfect-information research. It performed one PDF rendering/cropping probe. It did **not** build/run Arkham or Chiriboga, implement a Netrunner engine, measure bot strength, train a model, complete a 45-card crop map, or benchmark Jon's machine.

The project is technically plausible, but the central empirical uncertainty remains experienced-casual play in both roles on the accepted compute budget. Rust and Bevy do not remove that uncertainty. Existing Arkham mechanisms can reduce presentation and infrastructure work; they do not supply Netrunner's timing model or hidden-information strategy.

## Source basis

| Input | Inspected identity | Role / limitation |
| --- | --- | --- |
| [Uploaded constitution](../principles/principles-for-repos.md) | Revision 2026-09-30.1; SHA256 `f832a7241f9d1284ef32d444d26a69244f29068b855af197b9f032d9fd5ff4bb` | Exact supplied bytes, including original formatting |
| [CR PDF](https://access.nullsignal.games/CompRules/Null_Signal_Games_Netrunner_Comprehensive_Rules_v26.03.pdf) | 26.03; effective 2026-03-02; 99 pages; SHA256 `ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e` | Current rules basis at research time; targeted sections inspected, not a completed implementation conformance audit |
| [Rules hub](https://nullsignal.games/rules/comp-rules/) and [web rules](https://rules.nullsignal.games/) | Retrieved 2026-11-05 | Hub identifies current rules and NetrunnerDB's card-text authority; live URLs can change |
| [System Gateway product page](https://nullsignal.games/products/system-gateway/) | Retrieved 2026-11-05 | Exact starter/booster lists, printing information and PDF links |
| [Official card-data repository](https://github.com/Null-Signal-Games/netrunner-cards-json/tree/6ff880724741ea6ac062acf77e0cf979bb44e67e) | Commit `6ff880724741ea6ac062acf77e0cf979bb44e67e` | Source snapshot; inspect current authority again before implementation |
| [System Gateway JSON](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/pack/sg.json) | Git blob `423543db85b6a99eb3120099af7192ef44ce1e56` | 77 entries; printing metadata/card text, not executable rules |
| [Official rulings](https://github.com/Null-Signal-Games/netrunner-cards-json/tree/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings) | Same commit; 25 scoped files inspected | Verified-rules-team flags present; individual dates and old references still matter |
| [Arkham](https://github.com/joeloverbeck/arkham-unfolded/tree/d17075e873d0721f65840f9fdc338755e8a0de52) | Commit `d17075e873d0721f65840f9fdc338755e8a0de52` | Static code/docs audit, not native execution |
| [Chiriboga](https://github.com/bobtheuberfish/chiriboga/tree/77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39) | `dev` snapshot `77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39` | Comparative implementation; strength and current-rule fidelity not measured |

The [Corp guide](https://nullsignal.games/players/learn-to-play/learn-to-play-corp/) supplies the introductory deck/victory variant and concealment examples. The [Runner guide](https://nullsignal.games/players/learn-to-play/learn-to-play-runner/) explains the opposite perspective. The [learning landing page](https://nullsignal.games/players/learn-to-play/) and [run guide](https://nullsignal.games/players/learn-to-play/run-guide/) are teaching aids. Their diagrams and explanations cannot replace the CR's exact nested timing.

## Arkham audit

At the inspected revision, Arkham describes bounded Rust/3D experiments; a complete native Gathering playthrough remains outstanding. Its separate headless driver is relevant architectural evidence, not proof of a reusable general game engine. Its mission records Ubuntu/English, Rust-owned executable tooling, MIT code, and GitHub work control. Jon separately accepted those defaults here.

| Exact source | Observation | Extraction inference |
| --- | --- | --- |
| [Cargo.toml](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/Cargo.toml) | Rust 1.97.1, edition 2024, optional Bevy 0.20.0 tabletop; root graphics patch | Preserve headless isolation; compatibility work is concrete, not “use latest everything” |
| [Card shape](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_pieces/card_shape.rs) | Parametric face/body/outline geometry, Bevy imports, UV/winding tests | Strongest narrow presentation candidate; still needs both rendered consumers |
| [Record replacement](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/record_replacement.rs) | Bounded reads, sidecar lock, caller admission/recovery, staged replacement and previous record | IO lifecycle is separable; application schemas and compatibility decisions are not |
| [Camera](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_camera.rs) | Camera mechanics coexist with named investigator presets and Arkham areas | Extract movement/focus mechanics, not the entire module unchanged |
| [Pieces](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_pieces.rs) and [counters](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_counters.rs) | Geometry is mixed with chaos-bag, investigator, clue/resource, and harm-art concepts | Parameterize the observed rendering need; a common game-resource enum would import the wrong authority |
| [Assets](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/assets.rs) and [configuration](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/asset_config.rs) | Manifests, validation and external roots coexist with archive/Unity-font extraction | Reuse independently useful cache/verification operations; Netrunner's PDF adapter remains distinct |
| [Saved state](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/saved_state.rs), [continuity](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/continuity.rs), [driver](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/bin/driver.rs) | Current-state continuity and a headless entry point exist | Learn from the boundary; no basis to transplant Arkham state or replay semantics |
| [Pacing](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_pacing.rs), [UI](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_ui.rs), [health](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_health.rs), [capture](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/src/tabletop_capture.rs), [native tools](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/tools/src/native.rs) | Useful operational mechanisms exist alongside product assumptions | Candidate inventory only; no reason to extract them all before playable pressure |

Arkham's [play contract](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/docs/foundation/play-contract.md) and [mission](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/docs/foundation/mission.md) matter as much as its code. Replay was retired there on 2026-11-04; “reuse Arkham” therefore does not imply inheriting a replay feature. Netrunner's required history and current-state saves have their own acceptance contract.

The [vendored graphics patch record](https://github.com/joeloverbeck/arkham-unfolded/blob/d17075e873d0721f65840f9fdc338755e8a0de52/vendor/wgpu-hal/ARKHAM_PATCH.md) targets Linux/Vulkan, NVIDIA vendor `0x10de`, device `0x1e07` (RTX 2080 Ti), with observations on driver 615.78.08. It reports a staging-copy bottleneck and restored frame progress after changing eligible upload memory. Those are repository-reported measurements, not measurements from this investigation. Its upstream base is `wgpu-hal` 30.0.1 with preserved MIT/Apache-2.0 notices. The [Cargo patch reference](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html) and [workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html) explain why a dependency's patch does not propagate to application roots.

The shared-repository recommendation is therefore **selective extraction**, not an Arkham fork or immediate platform rewrite. The accepted two-consumer waiver handles the explicit generalization deficit; it does not make these candidates proven.

## PDF extraction evidence

Inspected [supplied A4 3× PDF](https://access.nullsignal.games/Gateway/English/English/SystemGatewayEnglish-A4%20Printable%20Sheets%203x.pdf), with the URL's analytics parameters removed:

| Property | Observation |
| --- | --- |
| Bytes | 98,206,517 |
| SHA256 | `1a66bf5c568ade62361ecc7d6e76d5cb460bef7e51ac4489e2c3694af3fd988b` |
| Pages / size | 27; A4, approximately 595.276 × 841.89 PDF points |
| Metadata | Created 2023-07-24; Affinity Publisher 2.1.1 |
| Structure | Whole-sheet raster images, approximately 280 ppi; text extraction yielded page separators rather than usable card text |
| Visually inspected | Page 1 cover; page 3 repeated 3×3 card grid; page 27 partial player-aid sheet |
| Probe | Page 3, bottom-left Cleaver; rendered at 300 dpi, rectangle x=123, y=2269, width=746, height=1033 pixels |

The JPEG crop was visually reviewed and contained the complete card face at readable inspection scale. The PNG viewing tool failed to decode its counterpart; this does not establish PDF or PNG corruption. No full mapping or automated recognition claim follows from this one crop.

Probe command used an existing external tool, not a project-owned preparation program:

```sh
pdftoppm -f 3 -l 3 -r 300 -x 123 -y 2269 -W 746 -H 1033 \
  -jpeg -singlefile system-gateway-supplied.pdf cleaver-crop-probe
```

These coordinates apply only to the inspected revision and rendering scale. Rendering above the embedded raster resolution cannot invent detail. An explicit crop recipe with reviewed printing IDs is a more reliable initial path than detecting arbitrary card layouts. Duplicate faces on the sheets do not specify deck counts. Covers and player aids are not playable cards.

The product page also offers a 1× PDF, which may reduce preprocessing work, but it was not substituted for Jon's supplied source or validated by this probe. Null Signal's [creator resources announcement](https://nullsignal.games/blog/announcing-the-creator-resources-portal/) and [media page](https://nullsignal.games/media/) show other resource routes; they do not remove the requested PDF extraction path.

[Visual assets](https://nullsignal.games/about/nsg-visual-assets/) licenses the included glyphs and symbols under CC BY-ND 4.0, explicitly excluding card art, frames, and backs. That narrow permission cannot be applied to all card images. Local preparation and a public artwork mirror are different distribution choices. The foundation uses original neutral sleeves/tokens so missing backs do not require an unexamined asset-license assumption.

## Rules interaction evidence

These examples informed the architecture and coverage obligations. They are paraphrases of inspected rulings/card text, not a replacement rulebook or a complete card implementation specification. Old ruling dates and references still require reconciliation with the selected CR. Other cards mentioned in upstream examples do not enter first-milestone scope.

| Source | Observed interaction | Why it matters |
| --- | --- | --- |
| [Brân 1.0](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/bran_1_0.json) | Its install procedure can trash protecting ICE, including Brân itself. After that installation completes, a trashed Brân's encounter ends and remaining subroutines do not resolve; the newly installed ICE corresponds to the next position. Ignored install costs do not imply a free later rez. | A cached ICE list or “resolve all subroutines” shortcut breaks a real starter card. |
| [AMAZE Amusements](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/amaze_amusements.json) | Its relevant ability can persist after being trashed during access only if active beforehand. Trashing an unrezzed copy does not activate it; other trash routes do not automatically persist. | Leaving play and delayed effects need precise source/lifetime handling. |
| [Conduit](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/conduit.json) | The run-end virus counter is too late to enlarge that run's breach. The access amount is established when the relevant breach ability resolves. | Run success, breach, access, and run end are different timing boundaries. |
| [Mayfly](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/mayfly.json) | Breaking with it creates delayed run-end trash; repeated uses can create repeated delayed instructions without trashing it prematurely. | Effects survive beyond the immediate paid ability and encounter. |
| [Overclock](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/overclock.json) / [Whitespace](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/whitespace.json) | Spendable credits hosted on Overclock are not credits in the pool. Whitespace's pool loss/threshold cannot simply inspect total spending power; its subroutines resolve sequentially. | Both rules and tactical cost calculations need resource provenance. |
| [Seamless Launch](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/seamless_launch.json) | Its target need not be advanceable and can be a Runner card if the installation-time restriction is met. Placing advancement counters differs from advancing. | “Sensible targets” cannot replace the legal target set. |
| [Creative Commission](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/creative_commission.json) / [VRcation](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/vrcation.json) | Additional click loss is an effect, so having no click left after playing the card does not make that play illegal. | Costs and instructions cannot share one blanket “can pay everything” filter. |
| [Wildcat Strike](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/wildcat_strike.json) | The Corp chooses the benefit; this is not a Runner optimization choice, even when one outcome is unattractive or cannot be fully realized. | Resolution can suspend for the opposite actor outside their turn. |
| [Jailbreak](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/jailbreak.json) | Its success-triggered draw precedes the breach; the run event remains in resolution until the run ends. | Draw knowledge and event lifetime affect timing, history, saves, and undo. |
| [Manegarm Skunkworks](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/manegarm_skunkworks.json) | The Corp has a relevant rez opportunity after the Runner's final ordinary jack-out opportunity and before the server-approach ability. | Collapsing the movement/approach windows removes tactical choices. |
| Funhouse, in [card data](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/pack/sg.json) | Its encounter ability arises before ordinary breaker paid-ability decisions. | Encounter-triggered effects are not printed subroutines. |
| [Send a Message](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/send_a_message.json) and CR §1.7 | A scored/stolen agenda can give the Corp a rez choice; an already reached victory condition ends the game before later resolution continues. | Terminal checks belong in effect resolution, not only at turn end. |

Other inspected ruling files at the same commit were `diviner.json`, `docklands_pass.json`, `karuna.json`, `leech.json`, `predictive_planogram.json`, `public_trail.json`, `red_team.json`, `smartware_distributor.json`, `tread_lightly.json`, `unity.json`, `urtica_cipher.json`, and `verbal_plasticity.json`. Together with the linked files above, these total 25. A ruling for Urtica contains a stale link to printing `30044`; the actual card snapshot identifies Urtica as `30045`. This is a concrete reason to reconcile canonical identities rather than treating hyperlinks as infallible data.

Predictive Planogram's older modal-choice ruling especially warrants comparison with current timing text before implementation. Coverage also extends to clauses that did not merit a special FAQ. Source-derived examples and engine-generated expectations serve different roles; generating all expected answers from the implementation would not independently verify fidelity.

## Bot research

The papers below address imperfect information, but none demonstrates experienced-casual Netrunner on Jon's PC. The engineering inferences in this table explain the selected starting point; they are not empirical results from this project.

| Approach and primary source | What the source establishes | Inference for this project |
| --- | --- | --- |
| Hand-authored tactics and heuristics; [Chiriboga source](https://github.com/bobtheuberfish/chiriboga/tree/77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39) | A complete browser implementation contains extensive role-specific heuristics and a run calculator | A useful baseline can encode actual Netrunner tactics; static code inspection cannot establish its strength |
| [Information Set Monte Carlo Tree Search, Cowling, Powley and Whitehouse (2012)](https://eprints.whiterose.ac.uk/id/eprint/75048/1/CowlingPowleyWhitehouse2012.pdf) | Information-set search addresses problems of naive determinization and was evaluated in several domains | A plausible bounded search family; does not by itself guarantee equilibrium play, good beliefs, or effective bluffing |
| [Counterfactual regret minimization, Zinkevich et al. (2007)](https://proceedings.neurips.cc/paper/2007/file/08d98638c6fcd194a4b1e6992063e944-Paper.pdf) | Regret decomposition enables approximate solutions of large extensive games; theoretical conditions matter | Strong reference for mixed strategies and small well-defined abstractions; whole-game Netrunner traversal/abstraction cost is unmeasured |
| [Neural fictitious self-play, Heinrich and Silver (2016)](https://arxiv.org/abs/1603.01121) | Combines learning best responses with averaged policies; reports approximate-equilibrium behavior in Leduc poker where ordinary RL diverged | Self-play stability is a real problem; simply training against the latest self is not a dependable strength plan |
| [Deep CFR, Brown et al. (2019)](https://proceedings.mlr.press/v97/brown19b.html) | Uses neural approximation for CFR instead of relying on manually tabulated abstractions | Avoiding a table does not eliminate simulation, representation, convergence, or training cost |
| [ReBeL, Brown et al. (2020)](https://arxiv.org/abs/2007.13544) | Combines reinforcement learning and search using public belief states, with results in imperfect-information games including poker | Valuable conceptual basis; poker results are not a turn-key Netrunner algorithm or a local compute estimate |
| [Ataraxos, Sokota et al. (2026)](https://www.nature.com/articles/s41586-026-11036-y) | Combines policy/value self-play, belief learning, and decision-time search; reports strong results across Stratego and other games | Shows that large hidden-state spaces need not preclude strength, while exposing training demands far above this project's accepted overnight budget |
| [OpenSpiel, Lanctot et al. (2019)](https://arxiv.org/abs/1908.09453) | A framework for algorithms and evaluation across perfect/imperfect-information games | Useful interface/evaluation reference; neither an available Netrunner rules engine nor evidence of a ready opponent |

The ISMCTS abstract/introduction and primary descriptions make the determinization problem explicit: an agent can accidentally evaluate plans as if hidden information were available later when it would not be. Netrunner adds asymmetric observations, memory of revealed cards, long plans, optional priority windows, and card-dependent legal actions. Search quality therefore depends on the model of information and opponents as well as simulation count. Even a fair current observation is insufficient if rollout opponents cheat.

The [ReBeL repository](https://github.com/facebookresearch/rebel) is archived and its released implementation covers Liar's Dice; it is not the complete poker system transplanted to arbitrary games. The [Ataraxos code](https://github.com/AtaraxosAI/stratego) uses a specialized training/simulation stack. Its primary Nature/PMC indexed methods report **16 H100 GPUs for one week** for reinforcement-learning models and **four H100s for four days** for the belief model. “Much cheaper than earlier research” is still not “one consumer PC overnight.” Direct Nature reading hit a cookie redirect; the primary indexed article/method text and repository were accessible. No training-cost extrapolation to Netrunner is claimed.

[Burn's backend documentation](https://burn.dev/books/burn/basic-workflow/backend.html), [API docs](https://burn.dev/docs/burn/), and [0.22 migration guide](https://burn.dev/books/burn/migrating-to-0.22.html) show a plausible Rust-native path to CPU/GPU learning. This supports language feasibility, not model choice, sample efficiency, memory fit, or achieved throughput. The foundation deliberately leaves the backend undecided until a concrete experiment needs one.

### Chiriboga specifically

The inspected [README](https://github.com/bobtheuberfish/chiriboga/blob/77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39/README.md) describes a client-side JavaScript game supporting Gateway and System Update 2021 and an abandoned earlier DQN attempt. Selected parts of [Corp AI](https://github.com/bobtheuberfish/chiriboga/blob/77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39/ai_corp.js), [Runner AI](https://github.com/bobtheuberfish/chiriboga/blob/77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39/ai_runner.js), and [run calculator](https://github.com/bobtheuberfish/chiriboga/blob/77c80b1f0f96a2af068f5d267cc0df1a1b0c9b39/runcalculator.js) show hand-authored priorities, card hooks, path calculations, and estimates for unknown ICE. JavaScript is not itself evidence of weak play or incorrect rules.

The comparative value is concrete: another implementation exposes awkward tactical cases and one author's division between search and card-specific knowledge. Its GPL-3.0 license differs from the proposed MIT project; source inspection is not a license to transplant its implementation into MIT code. This bundle contains none of that code. No benchmark or independent rules audit was performed, so neither “human-level” nor “untrustworthy” is an evidence-backed verdict here.

### Why evaluation is a separate problem

Win rate against one baseline can reward exploiting one weakness; self-play can cycle among mutually exploitable styles. A human-looking bluff needs strategic plausibility, not just random actions. Conversely, a tactically strong bot can feel repetitive or frustrating. These are different observations, which is why the foundation separates role-specific results, mistake categories, fairness, latency, and Jon's experience.

Future unseen decks create distribution shift even if every individual card is correctly implemented. Card-text embeddings, generic tags, or a larger neural network cannot establish transfer without new evidence. The existing first-scope corpus supports a bounded experiment, not a claim of arbitrary expansion readiness. No human-level claim, local-training estimate, or reusable learned policy is established by this research.

## Remaining empirical uncertainties

These are limits of the evidence, not an activated task list: current Arkham functionality at the later implementation date; cost and correctness of a real two-consumer extraction; native graphics behavior after that change; complete crop mapping and readable assets; rules fidelity across the full selected decks; headless simulation throughput; bot quality under the latency/training budgets; and the enjoyment and return-use experience on Jon's PC. The foundations preserve these uncertainties so future work can test them instead of inheriting invented certainty.
