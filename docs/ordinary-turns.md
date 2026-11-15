# Bounded ordinary-turn inspection

[Issue #12](https://github.com/joeloverbeck/netrunner-unfolded/issues/12), within
[#7](https://github.com/joeloverbeck/netrunner-unfolded/issues/7), extends the same
match, observation, legal-choice, controller and save path used by Brân.
This is the historical #12 engineering guide. Ordinary play now starts with
[complete introductory setup](introductory-match.md); these positions remain
bounded inspection inputs to the same transitions.

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

Paid windows advance automatically when passing is the human's only legal
choice. Scoring and every other meaningful choice remain explicit; a sole
target, payment, discard or steal option is never selected automatically.
Once the action phase opens, gain one credit. Play Seamless Launch on
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

The two mats have a visible table gap, and opposing hand cards are separated.
Credit and click pools each use one supplied-art token with their exact count
beneath it, including zero; positive advancement counts use the same style.
Counts stay attached while navigating and update after committed choices.

Stack, R&D, Heap and Archives are aligned piles whose height grows with the
observed card count. Empty zones have no pile. Stack and R&D show opaque backs;
Heap and Archives show their observed top card with its lawful face orientation.
Click Heap or Archives to inspect from the top, then use **Next card** or
**Previous card**. The position label includes the total. Browsing does not
reorder cards, change the saved game, or reveal hidden Archives faces to Runner.

Quit and repeat the same native command to resume. Choices are saved before
installing their resulting state. On an error, stop and preserve the active
and `.previous` records. The existing `--recover-from ACTIVE --save FRESH`
path validates a previous record before writing a new destination. Never
delete a good record to bypass a refusal. The engineering driver also has
`economy`, `discard` and `install` cases for reproducible checks.

The dock's **Undo** corrects your latest reversible choice, including its
resources, pending effects and history. Forced priority passes belong to that
choice. A draw, access, newly revealed card, random outcome or meaningful
opponent response seals correction; the dock explains refusal without naming
hidden cards. The same boundary survives save/restart. This is one-choice
correction, with no redo or arbitrary replay editing. The opponent always
computes from the current observation; stale decisions cannot survive rollback.

[Issue #21](https://github.com/joeloverbeck/netrunner-unfolded/issues/21) uses
the `ordinary-turns-10` save engine. Earlier ordinary-turn records are refused
unchanged: preserve them for their compatible build and prepare a fresh
engineering record when inspecting this version. Since #26, original Brân
checkpoint-v1 records are explicitly refused without mutation; current Brân
regressions use ordinary checkpoint-v2 continuations. Retain matching frozen
builds for the earlier records.

### Safe-correction check (#21)

Use the current [revision-9 artwork collection](turns-assets.md) and a fresh
external path. This reuses the existing draw/economy engineering position:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns draw20 --seed 20 --view runner --stop-after 2 \
  --save /absolute/fresh/undo21.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-9-assets \
  --save /absolute/fresh/undo21.json
```

Runner starts with 8 credits, 4 clicks and 4 cards. **Gain 1 credit** produces
9 credits and 3 clicks. Quit and repeat the native command, then choose **Undo**:
the table returns to 8 credits, 4 clicks and the earlier history. Next, **Draw 1
card** produces 5 cards and 3 clicks. Undo is now unavailable; its explanation
names the information barrier without identifying the private draw. Selecting
Undo leaves the committed state unchanged. Use the dock's scroll area when a
control is below the visible actions.

The check exercises the current controller and save path; it does not add a
playable mode or establish physical-input usability or owner acceptance. The
active issue owns native evidence and the existing inspection allowance. Stop
on a mismatching count, missing artwork or persistence error and retain the save.

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
pin is `84c857ecca4e6d22b2a5327e46b95642ea594acf`; the existing sibling
randomness migration and #9/#11 merge/pull gates are recorded in #12.
Numeric token-label creation and tracking now use the shared mechanism from
[shared PR #12](https://github.com/joeloverbeck/unfolded-tabletop/pull/12), after
[Arkham's migration](https://github.com/joeloverbeck/arkham-unfolded/pull/228)
passed its checks and native inspection and its main was merged and pulled.
Token meanings, zero-count policy, artwork and layout remain game-owned.

Source tests and offscreen rendering do not establish physical input,
readability, comfort or owner acceptance. The active issue owns the native
inspection result and remaining allowance. Full runs, complete deck setup,
later card families and opponent-strength claims remain outside this slice.
