# System Gateway introductory match

[Issue #22](https://github.com/joeloverbeck/netrunner-unfolded/issues/22) connects
ordinary setup to the existing match controller. Choose Corp or Runner, inspect
your five opening cards, and keep or redraw the whole hand once. The Corp decides
first. The exact teaching decks contain 34 Corp cards and 30 Runner cards,
excluding The Syndicate and The Catalyst. Six agenda points wins; flatline and
Corp draw failure also end the game. Booster decks and arbitrary deck imports
are not admitted.

Prepare the complete [introductory artwork collection](turns-assets.md), then:

```sh
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --assets /absolute/path/to/turns-assets --save /absolute/path/to/match.json
```

The default artwork root is
`${XDG_DATA_HOME:-$HOME/.local/share}/netrunner-unfolded/turns-assets`.
The role buttons appear in the existing bottom dock over the blue and red table.
No match or save is created before the required artwork has loaded and a role
is selected. `--seat corp` or `--seat runner` selects a new game's seat directly;
an existing save always restores its recorded seat.

Click a card for its upright face and current effective rules text. Close the
inspection to choose **Keep opening hand** or **Redraw all five · once only**.
The second hand is kept automatically. The Corp then begins turn 1, including
the mandatory draw. Both players begin with five credits. New human-Runner games
use [Corp-v2](corp-opponent.md), which considers its opening economy and affordable
defense before keeping or redrawing. New human-Corp games use [Runner-v1](runner-opponent.md)
for economy, breaker coverage and run planning from lawful observations.
Older introductory saves preserve their original opponent during migration.

Pan with WASD, orbit with right-drag, zoom with the wheel over the table, and reset
the camera with Home. Wheel over the bottom action dock to scroll long instructions
and choices. The human's cards face the near seat. Open History for the lawful
account of resolved choices. Undo is available only for the latest reversible
choice, before a draw, random outcome, reveal or opponent response.

Repeat the launch command to resume the same hand, suspended decision, seat,
history, opponent identity and future chance. A pending mulligan survives restart without drawing
another hand. At a completed match, **New match** returns to role selection.
The completed save remains unchanged until the new seat is selected; the next
game uses fresh chance and atomically replaces the save, retaining the completed
record as `.previous`. Quitting before choosing the next seat preserves the old
completed game. A live match has no reshuffle/reset control.

All gameplay runs offline. Fresh games use operating-system entropy through the
shared random module; the saved gameplay generator is separate from policy
decisions. Explicit seeds remain an engineering/test interface, not a difficulty
or ordinary-launch option.

Missing, changed or undecodable required images, either back, tokens or font stop
play before setup or save mutation. Failed saves leave the committed game intact
and freeze further choices until repair and reopening. Unsupported decks and
older/different game-content records are refused without reinterpretation.
Ordinary checkpoint-v2 saves retain their saved seat, chance and policy when their
state and undo boundary remain valid. The corrected public Runner-installation
boundary refuses an older save whose stored undo would conceal that disclosure.
There is no best-effort repair or overwrite: retain its matching frozen build.
Legacy checkpoint-v1 / `bran-continuation-1` saves are explicitly unsupported;
keep them with their matching builds. To recover the
previous valid record into a new path:

```sh
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --assets /absolute/path/to/turns-assets \
  --recover-from /absolute/path/to/match.json \
  --save /absolute/path/to/recovered.json
```

The Brân preset is retained as an engineering fixture and regression path through
the ordinary turn/run transitions and core/controller; it is no longer the ordinary native entry. Existing
`--turns --save EXISTING --seat corp|runner` engineering inspection remains
available for the bounded continuation guides.

Run `scripts/check` for the full input-independent check. `tests/setup.rs`
exercises exact card multiplicities by drawing all cards, private opening hands,
ordered mulligans, saved seat and chance, new-game replacement failure, and
complete seeded games in both seats with a midgame reopen. These are behavior
checks, not evidence of opponent strength or enjoyable play. Native evidence and
the owner acceptance boundary belong to #22 and its parent #7.

The open-table setup layout is Jon's selected VectorCraft alternative B. The
external editable source is `authoring/spec-7/setup-22/option-b-open-table.svg`
under the Netrunner data root, SHA-256
`3651d64681982e4a5d0ab8a5c0e40d48925db35a93ed318f0221389cee8bae6a`.
The runtime realizes that selected layout with the existing dock, seat colors,
card inspection and 3D hand; no animation was selected. Source rules and the
source refresh are recorded in [research evidence](research/evidence.md).
