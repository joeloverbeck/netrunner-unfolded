# Draw and economy across turns

[Issue #20](https://github.com/joeloverbeck/netrunner-unfolded/issues/20) adds
Creative Commission, VRcation and Verbal Plasticity through the existing match,
legal choices, player observations, baseline opponent and native table.
The `draw20` engineering witness preserves the exact 30-card Runner and 34-card
Corp inventory. It is preparation for a bounded check, not an ordinary setup
mode or complete-match admission.

Use [revision-9 artwork](turns-assets.md). This build writes `ordinary-turns-9`
saves with the basic-draw history needed even before Verbal Plasticity enters
play. Previous turn saves are refused and preserved without migration; retain
their compatible executable and artwork. The original Brân continuation is
unchanged.

## Prepare and launch

Create a fresh save and then open it as Runner:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns draw20 --seed 20 --view runner --stop-after 2 \
  --save /absolute/fresh/draw20.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-9-assets \
  --save /absolute/fresh/draw20.json
```

The native table skips empty timing windows automatically. Choose only the
actions offered; no extra Pass action is needed between ordinary actions here.
An existing save resumes its current state and ignores a new preparation seed
or step count. Use a fresh path for each independent checkpoint.

| Driver steps | State and next action |
| ---: | --- |
| 2 | Runner turn 2: 8 credits, 4 clicks, 4 cards. Take the basic draw: 5 cards, 3 clicks. |
| 4 | Install Verbal Plasticity from grip. It spends a click, then offers the existing installation choice; finish for 3 credits. |
| 5 | Installation pending: 8 credits, 2 clicks. Finishing spends 3 once, including after restarting here. |
| 7 | Verbal Plasticity installed: 5 credits, 2 clicks, 4 cards. Basic Draw still says 1 because a basic draw occurred before installation. After drawing: 5 cards, 1 click. |
| 9 | Play Creative Commission on the last click: pay 1 credit, gain 5; 9 credits, 0 clicks, 4 cards. It loses no extra click. Corp then takes its ordinary turn. |
| 23 | Runner turn 4: 9 credits, 4 clicks, 4 cards. Play VRcation: pay 1, draw 4, lose a remaining click; 8 credits, 2 clicks, 7 cards. |
| 25 | The first basic draw is still available. Draw 2: 9 cards, 1 click, 8 credits. |
| 27 | Play the second VRcation on the last click: 7 credits, 12 cards, 0 clicks. Draw 4 is unchanged by Verbal Plasticity; no extra click is lost. |
| 29 | Choose exactly 7 of the 12 cards to discard. Confirm remains unavailable until the required number is selected. |
| 33 | Four of those cards are selected. Restart preserves selection and the already resolved draws/payments. Select 3 more and confirm. |
| 37 | Discard complete: 5 cards, 7 credits, 14 cards in stack, 10 in heap. The driver reaches turn 5 at step 38 and stops without inventing a match result. |

Quit and reopen the same native command while installation or discard is
pending to inspect saved continuation. The opponent acts only from its own
observation; its credit-only turn in this witness matches the driver's itinerary
because the exposed Corp hand cannot afford its operation. In other positions
the baseline may choose a different lawful line.

Creative Commission and VRcation each cost one click and one credit. Their
additional click loss is a later effect, so last-click play is legal. Verbal
Plasticity costs three credits and is unique: a new copy trashes an older active
copy without refreshing the turn's first basic draw. Event draws, including
Jailbreak, neither receive nor consume that replacement. A short Runner stack
supplies only its remaining cards; an empty stack does not make the Runner lose.
Costs, legal choices and resolved consequences remain visible through the
existing inspection, action labels, counters and history.

The [source notes](research/evidence.md#draw-and-economy-source-basis-2026-11-15)
bind these expectations. Tests and rendered socket checks establish their
specific mechanical and runtime results; they do not establish human enjoyment,
opponent strength or owner acceptance. No additional human testing is requested
by this guide. Stop on a mismatching phase/count, missing art or persistence
error, and preserve the save.
