# Damage, jack-out and trap access

[Issue #16](https://github.com/joeloverbeck/netrunner-unfolded/issues/16), part of
[#7](https://github.com/joeloverbeck/netrunner-unfolded/issues/7), adds Diviner,
Karunā, Tithe and Urtica Cipher to the existing match, controller and native table.
Complete setup, other unsupported card families and complete introductory matches
remain unfinished. The driver prepares ordinary saved continuations; there is no
additional native scenario mode.

Use the [revision-5 artwork collection](turns-assets.md) and the
`ordinary-turns-5` save engine. Preserve earlier collections and saves: earlier
turn-engine records are refused without replacement, with no automatic migration.
The original Brân continuation remains supported separately.

## Prepare and resume

These source-derived inputs retain the exact introductory 30 Runner / 34 Corp
card multiplicities. Unsupported cards stay inactive in their owner's deck;
drawing or accessing one refuses the transition atomically. These inputs do not
admit complete decks to normal setup.

```sh
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns karuna16 --view runner --seed 42 \
  --stop-at 'Karunā' --save /absolute/fresh/karuna16.json
cargo run --locked --bin netrunner-driver -- \
  --engineering-turns damage16 --view runner --seed 42 \
  --stop-at 'Breach remote' --save /absolute/fresh/damage16.json
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --turns --seat runner --assets /absolute/path/to/revision-5-assets \
  --save /absolute/fresh/karuna16.json
```

Repeat the same native command to resume. To read a driver save without advancing
it, use its original case and add `--stop-after 0`. The saved chance state takes
precedence over a newly supplied seed. The engineering itinerary is separate from
the native opponent policy; every request still crosses the same validated match
boundary.

## Choices and settled consequences

For `karuna16`, the first Karunā subroutine has already trashed two random cards.
Runner has three cards in grip, two in the heap, eight credits and three clicks;
Corp has zero credits. **Jack out** ends this run and settles at the Runner action
phase with those counts unchanged. This is the nonterminal control.

Alternatively choose **Continue · resolve remaining unbroken subroutines**.
The second subroutine trashes two more cards, leaving one in grip and four in the
heap. The application settles at **Movement · continue or jack out**. Choose
**Continue run**, then **Access Remote 1 root card 1**. Access reveals the
unadvanced Urtica Cipher; its two net damage exceeds the remaining one card and
immediately flatlines Runner. Corp wins with Runner still at eight credits and
Corp at zero. There is no optional trash choice, later effect, or fabricated draw.
To check restart, quit at the initial Karunā decision and reopen the same save
before selecting a branch; the first damage is not repeated.

For `damage16`, Overclock initiated a run through Tithe and Diviner. Tithe dealt
one net damage and separately gave Corp one credit. Diviner trashed an actual
even-cost card and therefore did not end the run. At the saved remote breach,
Runner has two cards in grip, two in the heap, seven pool credits, five hosted
Overclock credits and three clicks; Corp has one credit. The root remains
concealed. Choose **Access Remote 1 root card 1**: Urtica's two net damage leaves
zero cards in grip and four in the heap, with no flatline. Choose **Trash accessed
card · pay 2 credits**, then **Pay 0 pool + 2 Overclock**. The run ends and the
application settles at Runner's action phase: seven credits, three clicks, zero
grip and five heap cards (including the finished Overclock); Corp has one credit.
Urtica is faceup in Archives. Remaining hosted credits return to the bank.

These are actual `Play::respond` settlement expectations, tested separately from
the driver's run-end stop. The existing action dock, card inspection, history,
pile counts and winner presentation carry the new behavior. No new layout or
animation is introduced. Missing or changed art refuses admission before a save
advances. Stop on different choices/counts, absent art or the issue's inspection
limit, preserving the save. This guide does not grant additional owner minutes.

## Source clauses and bounded evidence

The [official hub](https://nullsignal.games/rules/comp-rules/) was checked on
2026-11-14 and still selects CR 26.03. The retained PDF SHA-256 is
`ffd36071034aeae240cb319b13be1dae1d265c4598b44fbbfb90c86297a4523e`.
A fresh [NetrunnerDB API](https://netrunnerdb.com/api/2.0/public/cards) retrieval
has SHA-256 `ece969eac5159dfea2394cdeab1cf185cbe035567e318d2b08ddd982a3010606`
and agrees with the selected official card-data commit
[`6ff880724741ea6ac062acf77e0cf979bb44e67e`](https://github.com/Null-Signal-Games/netrunner-cards-json/tree/6ff880724741ea6ac062acf77e0cf979bb44e67e).

- Diviner 30046: one net damage, then inspect the printed cost of the card
  actually trashed. Odd ends the run; even and zero do not. Its verified rulings
  also treat X as zero; no currently admitted Runner card has an X cost.
- Karunā 30047: two net damage, then an explicit optional jack-out; continuing
  resolves the remaining unbroken subroutine, without another paid-ability window.
  Lethal first damage ends the match before offering that choice.
- Tithe 30073: one net damage and one credit are separate subroutines. A flatline
  during the first prevents the second from resolving.
- Urtica Cipher 30045: can advance while unrezzed. Access while installed deals
  one occurrence of `2 + advancement counters` net damage, including two at zero
  counters, before the optional two-credit trash. HQ, R&D and Archives access
  does not trigger the installed-only ability. Rezzing costs zero and is optional;
  the basic Corp policy preserves the unrezzed trap.

Controlling rules: CR 1.7.1–1.7.2b (immediate ending), 6.9.3 (subroutine order),
7.2–7.5 (access), 9.12.2 (aggregated quantities, including Urtica's worked example),
and 10.4.1–10.4.4 (random simultaneous damage). Verified rulings are
[`diviner.json`](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/diviner.json),
[`karuna.json`](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/karuna.json), and
[`urtica_cipher.json`](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/urtica_cipher.json).
The older Urtica ruling's link to 30044 is stale; the current card is 30045.

Tests use the accepted observation → legal action → validated transition seam,
with separate-process continuation and actual native dispatch. They cover the
mixed run, odd/even/zero and seeded random outcomes, advancement and installation,
zero/positive advancement damage, immediate lethal and exactly-enough-grip
controls, concealed roots and private R&D payment observations, invalid saved
references and unsupported remainder refusal. Public history names cards only
once their damage discard or access permits it. Policy inputs contain no hidden
chance state. These checks do not establish complete matches, universal absence
of information leaks, opponent strength or owner experience.

Existing shared RNG, saved-file replacement, rendering and Rust check recording
suffice. Inspected Arkham `cc3fbd13d4b9fcb5ecef4bf37007ad1275730ea3`, Mudsong
`8b4497a02deae1d33f6fbc3aef1f0e6cd9135657`, and shared library
`1f2ae285756f49ce1dc88868214727956308f8cc`; consumer dependency remains
`84c857ecca4e6d22b2a5327e46b95642ea594acf`. Damage timing, card clauses and
information rights are Netrunner-specific; no sibling code or shared contract
changed.
