# Engine and bots

## One authoritative game

Use a deterministic Rust rules core independent of Bevy rendering, assets, windowing, audio, network access, and wall-clock time. The GUI, headless driver, bots, and diagnostic tools use the same transitions. Begin with modules and concrete types; crate splits and generalized frameworks need actual consumers.

Grow bounded positions through [one match path](../adr/0001-one-match-path.md). A preset supplies starting state to that path; it does not own separate rules, UI, save semantics, or a parallel partial-game mode. Add match setup and broader play progressively, then retire the preset's ordinary-play entry when complete-match setup is available. Reusable regression fixtures may remain headless. This does not require implementing the full match lifecycle in the first increment.

The minimum execution boundary represents current authoritative state, the next decision and its actor, legal choices, explicit random outcomes, and a terminal result. Applying a choice validates its decision revision and actor. Invalid or stale choices leave state unchanged. Structured resolution events explain what changed and why; they are not a second rules authority. Card-specific Rust handlers compose only mechanisms already required by supported effects.

Resolution must suspend and resume nested decisions. Preserve costs, targets, trigger order, pending effects, priority passes, encounter/breach/access position, and effect durations as explicit serializable state. Do not collapse a run into one action or advance timing because an animation finished. Actual card objects and run positions need distinct identities so removal, installation, and zone changes remain meaningful.

Gameplay chance is seedable and separate from bot exploration/randomization. Store the relevant generator state and algorithm version. A diagnostic reproduction records the engine/content revision, starting state, chance basis, and decisions; it is not a promise of universal replay compatibility. Clone/fork and step the core headlessly for search. Measure throughput before optimizing storage or inventing a second simulator.

## Information boundaries

Bots and the human-facing application receive player-specific observations, authorized choice descriptions, and legitimate remembered information. They do not receive the actual hidden world. The authoritative controller alone owns hidden decks, hands, RNG state, and full save data. Debug omniscience is excluded from normal play and strength evaluations.

An observation includes public board/counts/history, that player's hand and permitted inspections, own deck composition, and the known fixed starter lists. Known composition is not knowledge of order or current location. Remembered revealed cards are qualified by subsequent shuffles and movements; persistent internal IDs must not track cards through an information-obscuring operation. Inspecting R&D as Runner does not automatically inform Corp of its top card.

Opponent facedown card types, hidden draws, target availability, labels, inspection text, resource annotations, and diagnostic logs can all leak information. Check the whole observation and exposed action set, not merely card textures. Paired states indistinguishable to the acting player should produce identical policy inputs and, with the same policy randomness/work budget, the same decision distribution. A simulated opponent also acts from its own information state.

## Bot approach

Recommendation: a hybrid whose components can be measured separately. Neither deep reinforcement learning nor plain perfect-information Monte Carlo search is an adequate default promise. [Research comparison](../research/evidence.md#bot-research) explains the evidence and limits.

1. **Legal baseline and tactics.** Build competent role-specific economic/scoring/run policies and exact local calculations for breaker sequences, affordable lines, immediate wins, and exposed lethal threats. Include resource opportunity cost, remaining clicks, future defense, and consequences of accessing unknown cards. Avoid always clicking credits or always choosing the cheapest current action.
2. **Beliefs and strategic variety.** Maintain plausible hidden-card distributions consistent with public play and prior observations. Use several opponent styles and stochastic choices among credible plans. Corporation bluffing should make protected agendas, economy, and traps plausibly overlap; random nonsense is not bluffing. Runner risk should change with score pressure, economy, and evidence.
3. **Bounded information-aware search.** Sample plausible worlds from beliefs, compare short plans, and aggregate under the acting player's information. Do not let a policy choose a different hidden-information-dependent action in each sampled world and then credit itself with all of those successes. This “strategy fusion” can make determinization appear much stronger than it is. Information-set methods are candidates, not correctness or equilibrium guarantees.
4. **Optional learned components.** Only after measured weaknesses and useful simulation throughput, try local parameter tuning or compact policy/value/belief models. Compare against the unchanged baseline and search-only version. A Rust training library such as Burn is a candidate, not an adopted dependency or evidence of feasible training time.

Search runs away from the render thread, observes cancellation, and always retains a legal fallback. Hard decisions have a 20-second ceiling; use much less for routine choices. Record wall time, work, fallback frequency, and decision quality. Use fixed-work runs for reproducible comparisons, and separate wall-time checks on Jon's actual machine. An overnight job gets an explicit duration, storage budget, checkpoint, and stopping condition; it does not automatically authorize subsequent nights.

## Evaluation and expansion

Evaluate Runner and Corp separately across both deck configurations. Use paired seeds, multiple styles and frozen previous policies, held-out positions/seeds, and tactical cases with independent expectations. Report outcomes, uncertainty, mistake categories, time, and information-boundary failures separately. Self-play win rate, one scalar rating, or beating random actions cannot establish experienced-casual strength. Compare components by removal or substitution to identify which changes actually help.

Jon's complete games establish whether this is enjoyable for him. A credible experienced-casual claim also needs full-game comparison with experienced players or an independently validated strength benchmark. Expert positions diagnose tactics; alone they cannot establish full-game human-level strength. Without suitable evidence, report that target as unverified. Do not quietly redefine success to mean “the bot finishes games.” No external tester recruitment or messaging is authorized by this document.

New card effects can require new planning features, tactical solvers, and learning representations. Admit future decks separately for rules support and bot quality; test unfamiliar combinations and hold out decks when making generalization claims. A card ID embedding or generic effect metadata does not supply understanding of new mechanics. Add abstractions only after real pressure. The headless core makes experimentation possible; it does not guarantee scalable competence.
