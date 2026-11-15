# Installed income across turns

[Issue #19](https://github.com/joeloverbeck/netrunner-unfolded/issues/19) adds Nico
Campaign, Regolith Mining License, Telework Contract and Smartware Distributor
through the existing match, player observations, native table and saved
continuation. Complete introductory setup remains unavailable. The engineering
witness retains the exact introductory inventory and uses ordinary installs and
turn transitions; it is not an additional ordinary-play mode.

Use the [revision-8 artwork](turns-assets.md). This build writes
`ordinary-turns-8` saves. Previous turn saves are refused and preserved without
migration; retain their compatible executable and artwork. Brân's original
continuation remains supported.

## Prepare and launch

Create a fresh checkpoint with the driver, then open it at either seat:

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns economy19 --seed 19 --view runner --stop-after 20 \
  --save /absolute/fresh/economy19.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-8-assets \
  --save /absolute/fresh/economy19.json
```

For another checkpoint use a fresh save path and the indicated `--stop-after`
count. An existing save resumes its current state; it does not reset to the
requested seed or preparation step. Use `--view corp` and `--seat corp` for Corp
checkpoints.

| Steps | Seat / turn | Visible state and next action |
| ---: | --- | --- |
| 0 | Corp / 2 | 10 credits, 3 clicks; Nico twice and Regolith in HQ. The native table advances to Action phase; install a campaign in a new remote, and choose whether to rez it. Installing costs a click; each rez costs 2 and loads its own hosted credits. |
| 20 | Runner / 3 | 5 credits, 4 clicks; Telework and Smartware in grip. Install Telework for 1 credit; its 9 hosted credits do not enter the pool. Its action takes 3 for a click, once per copy per turn. Install Smartware for 0; each activation places 3 from the bank for a click. |
| 28 | Runner / 3 | 7 credits, 1 click; Telework hosts 6, Smartware hosts 0. Activate Smartware: it hosts 3 while the pool remains 7. |
| 32 | Corp / 4 | 4 credits; each Nico hosts 9 and Regolith hosts 15. Choose which Nico income resolves first. Both mandatory payouts complete before the mandatory draw: pool 10, each Nico hosts 6. |
| 43 | Runner / 5 | 8 credits, 4 clicks; Telework hosts 6 and Smartware hosts 2. Smartware's automatic payout has taken 1 into the pool. Telework is available again. |
| 57 | Corp / 6 | 28 credits, 2 clicks; Regolith hosts 3. Take its last 3: pool 31 and Regolith moves faceup to Archives. |
| 64 | Runner / 7 | 14 credits, 4 clicks; Telework hosts 3. Take its last 3: pool 17 and Telework moves to the heap. |
| 74 | Corp / 8 | 32 credits; each Nico hosts 3. Choose either income first: both take 3 and trash themselves, drawing a card each, then Corp draws its mandatory card. Pool 38; neither campaign remains installed. |

The same buttons, card selection and textured credit counters show these
consequences. A sole mandatory payout resolves automatically. Multiple mandatory
payouts offer ordering, with no decline action. Smartware stays installed when
empty and can receive more credits while already loaded. Credits on these cards
cannot directly pay costs. Unrezzed Corp cards expose no title or hosted tokens
to the Runner.

Quit while an ordering choice is pending, then reopen the same native command:
the same obligations remain, and completing them pays each source once. Native
opponent actions may differ from the driver's deterministic click-credit
itinerary; both use the same legal transitions. The driver stops at turn 8's
action phase, not a fabricated match result.

Stop on a mismatching phase/count, missing art or persistence error and preserve
the save. Automated rules, restart and rendered-input checks do not establish
human readability, comfort, enjoyment or owner acceptance. Any human inspection
remains subject to the issue's existing allowance.
