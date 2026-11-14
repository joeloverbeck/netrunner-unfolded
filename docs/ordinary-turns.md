# Bounded ordinary-turn inspection

[Issue #12](https://github.com/joeloverbeck/netrunner-unfolded/issues/12), within
[#7](https://github.com/joeloverbeck/netrunner-unfolded/issues/7), extends the same
match, observation, legal-choice, controller and save path used by Brân.
Complete introductory setup is still unavailable. These engineering positions
exercise turns and installations; they do not offer ordinary runs or claim
complete behavior for the installed icebreakers and Docklands Pass during runs.

Prepare the [ordinary-turn artwork](turns-assets.md) first. The new collection is
separate from the existing Brân collection. Both backs, eight additional faces
and the supplied click/advancement token imagery are mandatory. Native loading
refuses a missing save or the wrong content family before creating a game.

An engineer can prepare a source-derived scoring position with the production
driver. Choose a fresh external save path:

```sh
cargo run --bin netrunner-driver -- --engineering-turns scoring --seed 42 --save /tmp/netrunner-scoring.json --stop-after 0
cargo run --features tabletop --bin netrunner-tabletop -- --turns --seat corp --save /tmp/netrunner-scoring.json --assets /path/to/prepared-turns
```

Pass the initial paid windows, then gain one credit. Play Seamless Launch on
the older Office in Remote 1, score it, then use the remaining click to gain
one credit. Expected result: two Corp points and eight Corp credits. Scoring
uses no click. A same-turn Office is not a valid Seamless target. The opponent
receives only its own observation and submits ordinary legal choices.

For Runner-side installation inspection, prepare a different fresh record with
`--engineering-turns runner`, then launch with `--seat runner`. The Runner is
near the camera and uses red counters; Corp is blue. Both seats can inspect
their own authorized faces upright. Unrezzed Corp roots and facedown Archives
retain opaque backs. Inspect a card or choose an explicit dock action; programs
can be trashed during program installation, and the final payment is shown
before commitment. Camera navigation does not change rules or consume clicks.

Quit and repeat the same native command to resume. Choices are saved before
installing their resulting state. On an error, stop and preserve the active
and `.previous` records. The existing `--recover-from ACTIVE --save FRESH`
path validates a previous record before writing a new destination. Never
delete a good record to bypass a refusal. The engineering driver also has
`economy`, `discard` and `install` cases for reproducible checks.

## Source and proof limits

Rules: CR 26.03, effective 2026-03-02, PDF SHA-256
`ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e`.
Card/ruling snapshot:
`6ff880724741ea6ac062acf77e0cf979bb44e67e`; the designated NetrunnerDB card text
was checked on 2026-11-14. See [research evidence](research/evidence.md).

The tests use source-derived expectations for CR 5.6/5.7 turns and mandatory
draw, 5.5 simultaneous discard, 8.5 installation/trash/cost order, 1.20 memory,
10.1 uniqueness, 1.18 advancement versus placement, and 1.17/10.3 agenda and
draw-failure checkpoints. The five economy/agenda cards have explicit tests
for costs, Seamless target timing including nonadvanceable/Runner cards,
score-versus-steal rewards and winning before on-score effects. A pending
Office access fixture exercises mandatory stealing; it does not supply run
initiation. Memory/uniqueness stress positions test the rule mechanism and are
not introductory deck admission.

`tests/turn_driver.rs` terminates and relaunches the actual production driver
at paid, install, discard and Seamless decisions, and compares uninterrupted
continuation with a different requested seed on reopen. Match saves retain the
shared SplitMix64 state and algorithm identity. Policy choice is deterministic
and receives no gameplay generator or private checkpoint. The shared producer
pin remains `987b77a38653f6e23cd707171e2c696c3ca59f54`; the existing sibling
randomness migration and #9/#11 merge/pull gates are recorded in #12. This is
actual Netrunner consumption, not a new shared extraction.

Source tests and offscreen rendering do not establish physical input,
readability, comfort or owner acceptance. The active issue owns the native
inspection result and remaining allowance. Full runs, complete deck setup,
later card families and opponent-strength claims remain outside this slice.
