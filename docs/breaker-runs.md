# Breakers and Overclock in the match path

[Issue #13](https://github.com/joeloverbeck/netrunner-unfolded/issues/13) extends the same match, controller and saved continuation used by ordinary turns. Engineering positions are inputs to that path; the native launcher still requires an existing prepared save. Complete introductory setup and the server/access continuation in [#14](https://github.com/joeloverbeck/netrunner-unfolded/issues/14) remain unavailable.

Prepare the [revision-2 artwork collection](turns-assets.md) into a fresh directory. Preserve earlier artwork and saves. This implementation uses the `ordinary-turns-2` save engine and the expanded content identity; earlier ordinary-turn saves are refused without replacement. There is no automatic migration. The original Brân continuation family remains separately supported.

To prepare a compact engineering checkpoint for native inspection:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns run13 --view runner --stop-after 24 \
  --save /absolute/fresh/run13.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-2-assets \
  --save /absolute/fresh/run13.json
```

The checkpoint starts at the Palisade encounter after the source-defined Overclock play, Brân replacement and normal three-credit rez. The Runner has seven pool credits, three clicks and five credits on Overclock. Select Cleaver's first-subroutine break and choose one hosted credit. After passing, continue to Whitespace; select Unity's **second** subroutine and again choose one hosted credit. Passing the encounter resolves Whitespace's first subroutine against the pool: four pool credits and three hosted credits remain. The bottom choices scroll when needed. Outside encounters, paid boost choices explicitly explain that their strength expires immediately.

After the final movement, the engineering position suspends at “Server approach · continuation unavailable.” It retains the run and Overclock credits and does not fabricate a successful run, access or match result. Quit and reopen the same save to inspect the preserved boundary. Jacking out earlier or allowing an end-the-run subroutine finishes Overclock, discards the event and returns unused hosted credits to the bank.

The independent headless witness uses the same controller:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns run13 --view runner --stop-after 100 \
  --save /absolute/fresh/whole-run13.json
```

Its final arithmetic is Corp zero credits, Runner four pool credits, three clicks and three hosted credits. This is the first four steps of #7, not its Manegarm/access continuation or full-match acceptance. The witness chooses the published worked line; the ordinary baseline instead uses only its player's observation and legal choices. Both humans retain optional install, trash, pass, payment, boost, selected break and jack-out decisions.

## Source and behavioral boundaries

Refreshed on 2026-11-14: the [official rules hub](https://nullsignal.games/rules/comp-rules/) selects CR **26.03**. The retained PDF SHA-256 is `ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e`. [Pinned System Gateway card data](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/pack/sg.json) was retrieved afresh (SHA-256 `d959c79535b5db78356dabc103275b3c4161d888478ecf75af4e2328be499765`), together with verified rulings for [Brân](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/bran_1_0.json), [Whitespace](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/whitespace.json), [Unity](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/unity.json) and [Overclock](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/overclock.json).

The source-derived obligations exercised in `tests/run_payments.rs`, `tests/turn_driver.rs` and the native controller tests are:

- CR 1.13.3: hosted credits are spending resources, not credits the Runner “has” or can lose from the pool.
- CR 3.9.5: strength and subtype constrain interface abilities; boosts last for an encounter and expire immediately outside one. Unity counts installed icebreakers and modifies only itself.
- CR 6.2.4–6.2.7 and Brân's rulings: declare the inward position before optional trash; trashing the encountered Brân ends its encounter only after installation completes. Any number of same-server ICE may be trashed. Installed ICE remains unrezzed and pays its later normal rez cost.
- CR 6.9 and 9.5: paid priority survives suspension; all unbroken subroutines resolve in order, without a paid window between them. The Runner selects Brân click breaks, breaker subroutines and pool/hosted splits.
- Whitespace's verified rulings: lose three pool credits first, then test whether the pool has six or fewer. Remaining Overclock credits do not avoid the second subroutine.

The tests distinguish source arithmetic, lawful projection, save admission, real process restart and native control dispatch. Automated checks do not establish physical-input usability or owner acceptance. No shared-library or GPU-pin change is required by this game-specific extension.
