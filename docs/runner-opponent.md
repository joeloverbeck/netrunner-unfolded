# Basic Runner opponent

[Issue #25](https://github.com/joeloverbeck/netrunner-unfolded/issues/25) adds
`RunnerV1` for new introductory games where the human plays Corp. Use the ordinary
[role selection and saved return](introductory-match.md). Existing saves retain
their saved opponent, including frozen RunnerV0. CorpV1 and both V0 sources and
identities remain unchanged.

The Runner receives only its current lawful observation. It has no Match, save,
gameplay seed, private diagnostic callback, future reveal or hidden-state clone.
It is deterministic and stateless: public central-run, successful-run, HQ-breach
and first-basic-draw history suffice for its current priorities. Public timing
facts are projected by the engine and survive the existing match save. No unused
memory or random stream is introduced. Its identity hashes its selector and the
public mechanics it consumes; the existing atomic save includes that identity.

| Decision | Behavior |
| --- | --- |
| Opening | Keep an economy card or use the once-only mulligan. |
| Economy/draw | Use efficient events and hosted payouts, value last-click Commission/VRcation, draw toward a useful grip, and retain funds for development. |
| Rig | Prefer missing subtype coverage, especially against exposed ICE; distinguish a permanent breaker from Mayfly's temporary coverage and memory cost. Successful runs make Carmen's discount relevant. |
| Pressure | Contest credible remotes, pressure HQ/R&D, favor fresh central information, use run-event benefits and Red Team income, and account for publicly possible traps. Facedown cards never acquire an inferred identity. |
| Encounter | Enumerate visible subroutine masks, compatible breaker boosts/breaks and Brân click alternatives. Use the remaining pool, temporary credits, clicks and grip across later known ICE and exposed Manegarm/Urtica. |
| Payment | Spend expiring Overclock credits before permanent credits. Whitespace's pool threshold and pool loss remain distinct from hosted credits. Compare Manegarm's two clicks with five credits and their actual temporary-credit opportunity. |
| Damage/exit | Avoid exposed lethal continuations. If a later obstacle makes the run impossible, avoid a wasted first payment; still prevent lethal damage in the current encounter before seeking an exit. Karunā's first resolved damage and optional jack-out are explicit public timing. |
| Mayfly | Price its delayed loss once per run, prefer a permanent alternative when otherwise comparable, and retain its use through later ICE until the real engine trashes it at run end. |
| Pending choices | Complete installation, replace lower-value rig cards when required, discard lower-value cards, resolve income, steal agendas, select useful accesses, trash economic assets/upgrades when affordable, and deliberately leave low-value accessed cards. |

The cost calculation is finite arithmetic over admitted visible mechanics. The
actual engine supplies legal actions and validates every transition. Shared
Netrunner-owned typed breaker quantities drive engine strength, compatibility,
boost and break capacity as well as plans; there is no second match simulator or
rules-prose parser. A plan's cost is a heuristic preference (pool credit 1, click
2, damage 2, newly lost Mayfly 4), not a quality verdict. Resource feasibility and
exposed lethal damage are separate from that preference. Nondominated resource
lines retain tradeoffs across the known remainder of a run. Subroutine masks
allow deliberate harmless Whitespace continuations and acceptable Tithe damage.

Unknown ICE is charged an uncertainty penalty, not simulated as its actual card.
A possible Brân inward installation remains uncertain. Plans do not predict Corp
choices or hidden trap locations. Conservatively breaking Diviner avoids relying
on which random card damage will trash. This is a basic policy with limited
horizon and heuristic risk preferences; it is not an optimal planner or evidence
of experienced-casual strength.

The unchanged [opponent worker](corp-opponent.md) isolates computation from the
render thread, checks controller/actor/revision/policy/epoch, preserves pure status
reads, supports cancellation and labelled legal emergency fallback, and commits
with the existing atomic save. Its nineteen-second budget retains headroom within
the twenty-second ceiling. Native status exposes redacted work/time outcomes.
No new screen or human inspection workflow is introduced.

`tests/runner_policy.rs` uses independently worked choices and real transitions:
four-credit Carmen versus disposable Mayfly, a first break invalidated by later
ICE/Manegarm, current and later lethal damage, seven pool credits plus Overclock
through Whitespace, delayed Mayfly trash, remote risk, draw/economy and changed
central history. Hidden permutations preserve exact inputs; revealed ICE is a
positive control. `tests/diagnostics.rs` extends ordinary-setup hidden controls to
RunnerV1, and `tests/opponent.rs` exercises the actual Runner worker/save boundary.
A dense exact-inventory workload also checks all thirteen exposed ICE with a
full mixed rig and ample resources against the response budget. These cases and
the bounded whole-game comparison establish only their stated
properties. The comparison's independent quality assessor covers imminent agenda
wins, unsafe Hub draw and exposed Karunā lethal; other choices remain unassessed.
The Karunā assessor now requires the actual unresolved continuation, so movement
after already-resolved damage is not falsely labelled a future lethal choice.
Report completion, legality/privacy, mistakes, unassessed choices, latency and
cost separately. Owner enjoyment and voluntary return remain in #26.

For #25 the predeclared comparison is RunnerV1 versus frozen RunnerV0, separately
against CorpV0 and accepted CorpV1, with development seeds 3/42 and first-use
held-outs 155921/196613. Both plans share the issue's sixteen-game, 120-second and
256-MiB allowance; every game has a 4,000-decision cap. Freeze first, retain failed
or censored arms, reproduce each retained record with the exact binary, and do
not tune after using the held-outs while retaining their held-out label. Actual
identities and results belong to the issue's evidence, not a new work tracker.

Source refresh, 2026-11-15: the [official hub](https://nullsignal.games/rules/comp-rules/)
still names CR26.03. GitHub API readback confirms the official card-data HEAD is
accepted `6ff880724741ea6ac062acf77e0cf979bb44e67e`; current pinned
[Mayfly](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/mayfly.json),
[Carmen](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/carmen.json),
[Whitespace](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/whitespace.json)
and [Manegarm](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/manegarm_skunkworks.json)
clauses were fetched directly. Direct NRDB requests returned403 and the rules-web
request timed out; those failures do not establish a source change. Existing
source precedence, admitted topology and independent engine cases remain binding.

Arkham `d0212e5` and Mudsong `4da6e8c` were inspected again. Arkham's artwork
selection/import workers own native dialogs and filesystem effects; they are not
observation-only policy proposals. No corresponding Mudsong policy worker was
found. #24's actual worker is reused unchanged. No new shared framework or sibling
migration is needed; existing shared persistence remains at its immutable pin.
