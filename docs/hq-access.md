# Manegarm, HQ access and later turns

[Issue #14](https://github.com/joeloverbeck/netrunner-unfolded/issues/14) extends the existing Match → Play → native table → saved continuation path through Manegarm, HQ/root access and later turns. This is the first difficult-path gate in [#7](https://github.com/joeloverbeck/netrunner-unfolded/issues/7), **not complete introductory play**.

The `hq14` engineering input contains exactly 30 Runner and 34 Corp cards with the introductory multiplicities, plus the two fixed teaching identities in observations. Its active cards and nine-step line are implemented. Unimplemented identities in the deck remainder are explicitly inactive: a draw or placement that would make one live is refused before committing the transition or replacing the save. Other servers' access continuations remain unavailable. There is no ordinary full-deck setup or claim that metadata admits these cards' effects. This input cannot be selected through the native launcher; it prepares a save for the same match path.

Use the [revision-3 artwork collection](turns-assets.md). The save engine is `ordinary-turns-3`; previous turn-engine records are refused without replacement, with no automatic migration. Preserve earlier collections and saves. All prepared gameplay is offline.

## Prepare and return

Create a fresh engineering checkpoint at the actual Manegarm decision:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns hq14 --view runner --stop-at Manegarm \
  --save /absolute/fresh/hq14.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-3-assets \
  --save /absolute/fresh/hq14.json
```

At Manegarm the Runner has **four pool credits, three Overclock credits and three clicks**; Corp has zero. Quit, then use the same native launch command to resume. All three choices remain: pay five credits (choosing pool/hosted sources), spend two clicks, or decline payment and end the run. This is a triggered payment, not a paid-ability window; there is no free pass through it.

The bounded native route is:

| Phase | Choose / inspect |
| --- | --- |
| Manegarm | **Manegarm · spend 2 clicks; continue run**. One click remains. |
| Breach HQ | **Access HQ root card 1**. Root access does not consume the HQ-card limit. |
| Accessing Manegarm | **Trash accessed card · pay 3 credits**. Leaving it is also legal. |
| Payment | Check **Pay for trash Manegarm Skunkworks**, then **Pay 0 pool + 3 Overclock**. |
| Breach HQ | **Access a random HQ card**. The seeded case accesses Hedge Fund then Office; the engine chooses randomly from eligible cards without replacement. |
| Hedge Fund access | **Leave card · finish access**, then access the remaining HQ card. |
| Office access | **Steal Offworld Office · 2 agenda points** is mandatory. Corp receives no scoring reward. |
| Run ends / paid window | Overclock leaves play. **No more abilities · pass** declines optional boosts. When the action phase arrives, **Gain 1 credit · 1 click**, reaching five. |

The dock scrolls. Check the phase and the payment's purpose before paying. Stop if the expected card/phase or arithmetic differs, if an asset is missing, or at the inspection allowance; preserve the save. Do not try to repair the rules manually. Physical-input comfort and owner acceptance require actual use; automated UI dispatch and render captures do not establish them.

The ordinary Corp baseline can choose a different legal economic line after this route: after Seamless/Office scoring it may play Hedge Fund, reaching **11** rather than the source witness's **8**. That is a lawful choice, not the nine-step witness. No native policy receives hidden state or a special acceptance-case autopilot.

Archives and Heap use raised physical layers, with the last card in the existing observation on top. Each card retains its inspection handler and an exposed edge; the pile's horizontal spread is capped. Archives reports the public faceup/facedown counts and continues to show backs for facedown cards, including from the Corp seat. Inspecting those cards uses the same seat-limited observations as before. This repairs intersecting card bodies after Brân and Manegarm are trashed; it changes neither discard ordering nor save state and introduces no pile browser.

## Independent worked line and boundaries

The driver follows all nine source-defined steps through the same legal choices, including the intentionally selected final Corp credit action:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns hq14 --view runner --stop-after 1000 \
  --save /absolute/fresh/full-hq14.json
```

It stops at the next Runner turn's HQ breach: Runner two pool credits, Corp eight credits, two agenda points each, Brân in Archives, Palisade still installed, and Manegarm absent. The exact input counts and source arithmetic are independently asserted in `tests/hq_access.rs`. `tests/turn_driver.rs` terminates and launches separate OS processes at Manegarm and nested access/payment decisions, comparing resumed and uninterrupted outcomes. Saving preserves chosen random outcomes and both players' lawful history.

HQ breach history counts actual breaches, not attempted runs, and resets each turn. Docklands fixes the first breach's HQ limit at two; later breaches in that turn get one. The Runner can order root access before, between or after HQ accesses. Root ordering uses public physical slots; hidden HQ candidates never become selectable card identities. Accessed HQ cards remain known during the breach, with no persistent tracking identifiers in the access projection; afterwards the history retains the legitimate observation without exposing current hidden positions.

Manegarm can rez in the final non-ICE rez window after the last jack-out opportunity (CR 6.9.4e), then triggers on server approach (6.9.4g). Payment alternatives remain separate and affordable splits are explicit. Brân retention/declined installation, unaffordable rez, jack-out, unbroken Whitespace and Manegarm refusal retain their legal outcomes. Inactive remainder refusal does not reveal the card's name and does not mutate current or previous save records. It is a scope boundary, not a match ending.

## Shared rendering basis

The discard repair refreshed clean Arkham `833254635c59b82b36c4a9eb02acc33cd1485dc4`, Mudsong `98bc23749c96bb55741b280e37218b8d99887273`, and shared checkout `1f2ae285756f49ce1dc88868214727956308f8cc`. Arkham selects its top discard with a count; Mudsong also uses the shared physical-card body and has a separate pile inspection workflow. Netrunner already consumes that shared body at pinned revision `84c857ecca4e6d22b2a5327e46b95642ea594acf` (the inspected mesh implementation matches). Its zone placement and face/inspection policy stay local: only card elevation and bounded spread were missing, so this repair requires no shared primitive, copied sibling renderer or new browser.

## Source basis

Refreshed 2026-11-14 against the [official rules hub](https://nullsignal.games/rules/comp-rules/), which still selects **CR 26.03**. The freshly retrieved [PDF](https://access.nullsignal.games/CompRules/Null_Signal_Games_Netrunner_Comprehensive_Rules_v26.03.pdf) retains SHA-256 `ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e`. Governing clauses include 6.9.3–6.9.5, 7.1–7.5, 8.5.6b, 8.5.10 and 8.5.13a, plus mandatory stealing and checkpoint timing.

The [pinned official card snapshot](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/pack/sg.json) remains commit `6ff880724741ea6ac062acf77e0cf979bb44e67e`, freshly retrieved SHA-256 `d959c79535b5db78356dabc103275b3c4161d888478ecf75af4e2328be499765`. Live NetrunnerDB API titles, text, costs, trash costs, strengths, advancement, agenda points and memory matched for Manegarm, Docklands, Office, Seamless, Brân, Whitespace, Palisade, Overclock, Cleaver and Unity.

Verified pinned rulings for [Manegarm](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/manegarm_skunkworks.json), [Docklands](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/docklands_pass.json), Brân and Whitespace were retrieved afresh. Manegarm's older step references reconcile to CR 6.9.4e/g; Docklands counts the actual breach. Rules implementation remains local to Netrunner; randomness, saved-record replacement and rendering continue to consume the already-pinned shared revision without a new extraction.
