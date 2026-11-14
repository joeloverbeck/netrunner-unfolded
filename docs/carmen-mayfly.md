# Carmen and Mayfly in the same match

[Issue #17](https://github.com/joeloverbeck/netrunner-unfolded/issues/17) adds these
two breakers to the existing match, saved continuation and native table. Complete
introductory setup, the remaining cards and stronger opponents remain unfinished.

Prepare the [revision-6 artwork](turns-assets.md) in a fresh directory. This
revision uses save engine `ordinary-turns-6`: earlier turn saves are refused
without replacement or implicit migration. Preserve their binaries, collections
and saves. The separate original Brân continuation remains supported.

The engineering input retains the exact introductory 30 Runner / 34 Corp card
multiplicities. Unsupported remainder stays inactive in its owner's deck. The
driver earns a successful Archives run through the normal rules before offering
Carmen's discounted installation; no fixture flag supplies the discount.

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns breakers17 --view runner --stop-at Installation \
  --save /absolute/fresh/breakers17.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-6-assets \
  --save /absolute/fresh/breakers17.json
```

At this checkpoint, Carmen costs **3 credits**, after this turn's successful run.
Finish installing it. Unity, Mayfly and Carmen use all four memory units but are
three icebreakers. Play Overclock and run HQ. Advance paid windows using **No more
abilities · pass**; use **Continue run** at movement. Only select a boost when the
heading says **Encounter**: otherwise its strength expires immediately.

The mixed-ICE route, outermost first:

1. **Karunā, strength 3:** Carmen starts at 2. Boost it once for +3, paying
   **1 pool + 1 Overclock**. Break each subroutine separately with Carmen, paying
   **0 pool + 1 Overclock** each time. Choose **Finish encounter**, then continue.
2. **Palisade, strength 2:** Mayfly starts at 1. Boost it once, then break its
   subroutine, paying **0 pool + 1 Overclock** for each ability. Its warning now
   says it will be trashed at run end. It remains in the rig after this encounter.
3. **Diviner, strength 3:** Unity starts at 1. With three installed breakers, one
   boost raises it to 4. Boost and break the subroutine, paying one pool credit
   each. Finish the encounter. At movement, Mayfly is still installed; quitting
   and reopening the exact same save retains its delayed trash and all choices.
4. Continue to the empty HQ breach. Mayfly is trashed when the run ends. Unity
   and Carmen remain, using two memory units. Following these payments leaves
   **13 pool credits**, no hosted credits, no damage and a completed successful
   run. Ending that run by a legal jack-out also trashes a used Mayfly.

Alternative payments and breakers remain legal choices. Carmen can only break
sentry subroutines; Mayfly can break any admitted ICE subtype at matching
strength. A boost does not break anything, and boosting Mayfly alone does not
schedule its trash. Every boost here lasts only through its current encounter.
Carmen costs 5 again next turn; an unsuccessful run later in the same turn does
not erase an earlier successful run. Both breakers are nonunique. Installing
Mayfly needs two free memory units, with explicit program-trash choices if needed.

The native inspection is limited to the active issue's accepted five cumulative
minutes. Stop at completion, a defect, or that limit. Automated checks do not
establish physical input, readability, comfortable play or owner acceptance.
On a save or asset refusal, preserve the active and `.previous` files and stop.

## Source and verification boundaries

Refreshed 2026-11-14: the [official rules hub](https://nullsignal.games/rules/comp-rules/)
still names CR 26.03. Its [PDF](https://access.nullsignal.games/CompRules/Null_Signal_Games_Netrunner_Comprehensive_Rules_v26.03.pdf)
has SHA-256 `ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e`.
The [official card snapshot](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/pack/sg.json)
remains `6ff880724741ea6ac062acf77e0cf979bb44e67e`; its Gateway JSON hash is
`d959c79535b5db78356dabc103275b3c4161d888478ecf75af4e2328be499765`.
The designated current-text API records for
[Carmen](https://api.netrunnerdb.com/api/v3/public/cards/carmen) and
[Mayfly](https://api.netrunnerdb.com/api/v3/public/cards/mayfly) agree with it.
The [NSG-verified Mayfly ruling](https://api.netrunnerdb.com/api/v3/public/rulings?filter%5Bcard_id%5D=mayfly)
confirms repeated breaking creates delayed abilities, with later resolutions
doing nothing after the original program has left play. No Carmen rulings were
returned. Printed artwork is not the rules authority.

Independent expectations use CR 3.9.5b–h (strength and interface restrictions),
6.9.5a (success before breach), 9.1.4 (source identity), 9.6.13 (delayed abilities),
9.10.4 (duration outside an encounter), memory and uniqueness. The hostile case
combines a damage-capable sentry, barrier and code gate with explicit payments,
same-turn history, changed rig count, saved continuation and run-end cleanup.
The tests also cover unsuccessful runs, turn reset, multiple Mayfly instances,
ordinary and Karunā jack-out, ICE termination, Manegarm refusal and immediate
flatline stopping further resolution. No unsupported card is introduced to
manufacture a source-lifecycle case.

`tests/breakers.rs` exercises the accepted observation → choice → transition
boundary and checkpoint admission. `tests/turn_driver.rs` restarts the production
driver across this route; the native controller test uses the actual buttons and
save path. The basic policy receives lawful observations and only chooses offered
actions; these checks make no strategic-strength claim. Existing shared save,
randomness and rendering mechanisms remain pinned at
`84c857ecca4e6d22b2a5327e46b95642ea594acf`, without a new shared extraction.
