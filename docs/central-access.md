# Central access and agenda checkpoints

[Issue #15](https://github.com/joeloverbeck/netrunner-unfolded/issues/15), part of [#7](https://github.com/joeloverbeck/netrunner-unfolded/issues/7), extends the same Match → Play → native table → saved continuation path through R&D and Archives. It adds Jailbreak, Send a Message and Superconducting Hub. Complete setup, the other card families, other remote families and complete introductory matches remain unimplemented. These engineering inputs prepare normal saves; the native launcher has no separate scenario mode.

Use the [revision-5 artwork collection](turns-assets.md), which includes all three new faces. The engine identity is `ordinary-turns-5`. Earlier turn-engine saves are refused without replacement; preserve those saves and collections. There is no automatic migration. Prepared play is offline.

## Prepare a checkpoint

The winning case and its control retain the exact introductory 30 Runner / 34 Corp card multiplicities. Unsupported remainder remains inactive in the decks. A transition that would draw or access an unsupported card is refused atomically, without its identity or a changed save. Metadata and artwork do not admit the rest of the decks.

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns winning15 --view runner \
  --stop-at 'Accessing Send a Message' --save /absolute/fresh/winning15.json
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns central15 --view corp \
  --stop-at 'Send a Message' --save /absolute/fresh/central15.json
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns hub15 --view corp \
  --stop-at 'Superconducting Hub' --save /absolute/fresh/hub15.json
```

For any prepared save, use the corresponding seat:

```sh
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-5-assets \
  --save /absolute/fresh/winning15.json
```

Repeat the same launch command to resume. The driver can also read the checkpoint without advancing it by adding `--stop-after 0`. All choices still go through the production observation/action boundary. The engineering driver explicitly selects Jailbreak and R&D for the worked line; this is not a special native opponent or a strength claim.

## Expected decisions and consequences

| Prepared save | State and choice | Consequence |
| --- | --- | --- |
| `winning15`, Runner seat | Runner has 3 points, 5 pool credits and 3 clicks. Jailbreak has drawn Sure Gamble. The accessed Send a Message has the sole action **Steal Send a Message · 3 agenda points**. | Runner reaches 6 and wins at the checkpoint. No Corp rez choice and no second access occur. Palisade remains unrezzed. |
| `central15`, Corp seat | Runner has stolen Send a Message and has 5 points. Corp has 0 credits. Choose **Rez Palisade · HQ ICE 1 · free (Send a Message)** or **Do not rez ICE**. | The selected optional effect resolves without charging credits, then the same R&D breach continues. The ordinary Runner baseline accesses Hedge Fund and leaves it, then finishes its turn by playing Sure Gamble and gaining two credits. At the next Corp action phase: Runner 11 credits / 5 points, Corp 0 credits / 0 points / 3 clicks, HQ 1, R&D 30. Jailbreak is in the Heap. |
| `hub15`, Corp seat | Corp has completed its mandatory draw and scored Superconducting Hub for 1 point, with 1 card in HQ and 2 in R&D. Choose **Draw 2 cards · Superconducting Hub** or **Do not draw cards**. | Drawing adds Seamless Launch and Hedge Fund: HQ becomes 3 and R&D 0. Declining leaves HQ at 1 and R&D at 2. The application settles at the Corp action phase; there is no further mandatory draw in this turn. |

The existing table, action dock, inspection panel and victory presentation are reused. There is no new layout, animation or design-selection requirement. Missing or changed required artwork refuses native admission before the save advances. Stop and preserve the save if a phase, card, payment, count or loaded image differs. These instructions are a reproducible technical handoff, not authorization for additional owner inspection; that allowance is governed by the issue.

## Rules and limits exercised

- Jailbreak can run only HQ or R&D, costs no credits and one action click, draws on success, and adds one central access. An empty Runner stack does not cause defeat. The event stays in play through the run. Docklands and Jailbreak combine for three HQ accesses on that turn's first HQ breach; root access does not consume that count.
- R&D offers the next eligible card from the top, including after leaving, stealing or trashing a previous card. The Corp cannot inspect that card or learn its type from the access phase, history or trash-payment label. Root choices are independent of the central access limit.
- Breaching Archives turns its discard pile faceup. The Runner chooses discard or root candidates. A root card trashed into Archives becomes a new candidate, as required by CR 7.4.5; a card already in Archives cannot be trashed during access. Empty central zones complete normally.
- Send a Message offers the Corp an optional free rez of one installed unrezzed ICE on score or steal. Winning checkpoints occur before that choice. With no eligible unrezzed ICE, the empty effect is skipped automatically. Superconducting Hub offers a two-card draw only on scoring and adds two maximum hand size per copy only in the Corp's score area. An optional draw from insufficient R&D can lose the game; declining cannot. The baseline declines when fewer than two cards remain.
- New agendas use their actual advancement requirements and point values; installing another agenda in an occupied remote requires trashing the existing agenda. New choices and event references survive checkpoint/resume; malformed Jailbreak references are refused.

`tests/central_access.rs` contains independent source expectations and paired hidden-state cases. `tests/turn_driver.rs` stops and launches separate OS processes at the new agenda decisions. The native dispatch test uses the actual action buttons through the winning steal. Headless and automated GPU evidence do not establish physical-input comfort, full-match fidelity or owner acceptance.

## Source and reuse basis

Refreshed on **2026-11-14**: the [official rules hub](https://nullsignal.games/rules/comp-rules/) still selects [CR 26.03](https://access.nullsignal.games/CompRules/Null_Signal_Games_Netrunner_Comprehensive_Rules_v26.03.pdf), SHA-256 `ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e`. Controlling clauses include 1.17.2–1.17.7, 3.2.3, 5.5, 6.9.4–6.9.6, 7.1–7.5 and 10.3. The introductory six-point threshold remains the accepted teaching variant. The independent [winning-steal/control case](research/evidence.md#next-match-source-audit-2026-11-14) predates this implementation.

The pinned official card snapshot remains `6ff880724741ea6ac062acf77e0cf979bb44e67e`: [Jailbreak](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/jailbreak.json), its [verified rulings](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/jailbreak.json), [Send a Message](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/send_a_message.json) and [Superconducting Hub](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/superconducting_hub.json). Fresh NetrunnerDB effective text/cost/advancement/points match these sources. Jailbreak's older step 5a reference reconciles to CR 6.9.5a. No source ambiguity emerged for the admitted cases.

Inspected clean sibling checkouts: Arkham `cc3fbd13d4b9fcb5ecef4bf37007ad1275730ea3`, Mudsong `98bc23749c96bb55741b280e37218b8d99887273`, shared library `1f2ae285756f49ce1dc88868214727956308f8cc`. Netrunner continues consuming already-shared randomness, saved-file replacement and rendering at immutable dependency `84c857ecca4e6d22b2a5327e46b95642ea594acf`. Card-specific breach candidates, information rights and agenda timing remain local. No new shared mechanism or sibling mutation was needed.
