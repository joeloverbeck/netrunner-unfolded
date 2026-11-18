# Brân at HQ

Historical bounded-entry guide and current engineering reference. Ordinary
native launch now uses [introductory match setup](introductory-match.md), with
the complete artwork collection. The old native launch/recovery examples below
describe the accepted #1 build; retain it for old Brân saves. The current
asset-preparation and headless regression commands remain valid. Since #26, the
Brân fixture initializes ordinary turns and reaches the subroutine through actual
run transitions. Its current saves use checkpoint v2 / `ordinary-turns-10`;
legacy v1 / `bran-continuation-1` records are refused without mutation. Keep the
matching frozen build for those records. There is no finite-run reset control.

The historical #1 build implements the accepted bounded position in [issue #1](https://github.com/joeloverbeck/netrunner-unfolded/issues/1).
You play the Corp against a Runner that continues whenever allowed. Decline the
installation, install Palisade and keep Brân, or install and trash Brân before
deciding whether to pay three credits to rez Palisade. The run finishes; this
entry does not continue into unsupported turns or declare a match winner.

## Prepare and launch

Use Rust 1.100.0, Ubuntu's Vulkan/X11 libraries, Poppler (`pdftoppm`), and the
recorded DejaVu Sans font from `fonts-dejavu-core`. The selected PDF is the
[System Gateway English A4 3× sheet](https://access.nullsignal.games/Gateway/English/English/SystemGatewayEnglish-A4%20Printable%20Sheets%203x.pdf).
Keep it and the supplied Android Netrunner resource collection outside Git.
The exact source identities, crop coordinates and resulting image hashes are
in [`resources/bran-assets.json`](../resources/bran-assets.json).
The bounded [ordinary-turn positions use a separate collection](turns-assets.md);
preparing it does not replace or reinterpret this Brân collection.

```sh
cargo run --locked --bin netrunner-assets -- prepare \
  /absolute/path/to/SystemGatewayEnglish-A4.pdf \
  '/absolute/path/to/Android Netrunner Resources' \
  /absolute/path/to/corp-back.jpg \
  /absolute/path/to/bran-assets
cargo run --locked --bin netrunner-assets -- check /absolute/path/to/bran-assets
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --assets /absolute/path/to/bran-assets --save /absolute/path/to/match.json
```

The default runtime collection is
`${XDG_DATA_HOME:-$HOME/.local/share}/netrunner-unfolded/bran-assets`.
Preparation preserves the original inputs and any existing output directory.
An identical complete collection is reused; incomplete or different output is
refused. Choose a fresh folder when the recipe changes. No credentials, network
service or LLM is used during gameplay. `--save` selects an open-or-create
autosave; repeat the same launch command to resume. Omitting it keeps the run
in memory only. Save files belong outside the repository.

### Resume and recovery

With `--save`, each legal decision is saved before it appears on the table,
including the fixed Runner's decisions. Quit at the initial installation choice,
during keep/trash selection, or at the later rez choice; relaunch with the same
assets and save path to continue with the same resources, choices and lawful
history. The current bounded run has no random outcomes or background bot work.
Full matches and their chance state remain later work in [#7](https://github.com/joeloverbeck/netrunner-unfolded/issues/7).

The previous committed decision is retained in `match.json.previous`. Unsupported
formats, engine/content identities, invalid game state and damaged records are
refused. The selected artwork must also be verified and loaded before any game
is restored or an initial save is created. Repair artwork and relaunch when
admission fails. Save failure freezes further choices at the last committed
decision; close and reopen after repairing the reported problem. A second
process cannot replace decisions written after it opened the record.

To recover a valid previous decision after damage, use a **new, unused** path:

```sh
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --assets /absolute/path/to/bran-assets \
  --recover-from /absolute/path/to/match.json \
  --save /absolute/path/to/recovered-match.json
```

This restores `match.json.previous`, so the most recent committed decision may
be lost. It preserves both source files for diagnosis; future launches use
`--save /absolute/path/to/recovered-match.json` without `--recover-from`.
Recovery refuses an occupied destination or an invalid previous record.
The shared replacement syncs staged files and retains the previous copy; it
does not promise a multi-file transaction or survival of every power-loss case.

Four card faces are cropped at 300 DPI, using the per-card dimensions in the
recipe: 744×1032 pixels, or 743×1032 for The Syndicate. These bounds exclude
the neighbouring printing and white page strip. The selected
`credit-advancement-light.png` is a 512×256 atlas: its **right** 256×256 region is
the credit face; the left advancement face is unused. The circular face uses
opaque alpha on a solid cylinder with a separately colored edge. The Corp back
is the verified JPEG referenced in `corp_back_source`, also used by the supplied
TTS mod. Supply that exact external file as `CORP_BACK_JPEG`; preparation decodes
it to the opaque `backs/corp.png` without resizing or modifying the original.
The `bran-run-v2` recipe replaces the earlier generated sleeve, so prepare it in
a fresh folder; the previous collection is preserved. The verified DejaVu Sans font is copied to
`ui.ttf`; it supplies Brân's accented name and readable interface punctuation.

### Locate an existing source before declaring artwork missing

Start with the supplied collection and its source/selection records. The token
collection and printable PDF do not contain card backs, but that does not establish
that the owner's wider resources lack them. Check the installed Tabletop Simulator
mod's saved metadata next: `Mods/Workshop/3670075370.json` contains `CustomDeck`
entries with `FaceURL` and `BackURL`. Match the relevant deck/side to its source URL;
a cache filename alone does not identify a card or prove it is the right back.

Resolve that URL in the same installation's `Mods/Images` cache. In the inspected
Steam Snap installation, the data root was
`~/snap/steam/common/.local/share/Tabletop Simulator`; cache names used the URL's
letters and digits followed by an image extension. The Corp-back URL is already
recorded as `corp_back_source` in the recipe. Its matching cache JPEG was the
source of the current back, independently of the PDF; no new TTS launch was needed.

Decode and visually inspect the candidate, then verify its byte count and SHA-256
against the recipe before preparation. Preserve the source and keep image payloads
outside Git. If the selected source cannot be found or verified, report the exact
missing role and the locations examined before requesting another image. This is
an optional preparation lookup; normal play uses only the prepared collection.

Before constructing the match, the application checks every required file,
decodes the images, loads the exact images into the renderer, observes completed
GPU uploads, and checks that the text renderer registered the required font.
Missing, changed or failed resources stop play with a repair message. If a
required resource becomes unavailable after admission, the last position freezes
and further decisions are refused. Repair the external collection and restart.

## Use the table

The Corp is nearest the camera. HQ is in the centre, Archives to the left and
R&D to the right; the green highlight marks the run. ICE is sideways and ordered
inward toward HQ. Unrezzed installed ICE shows the Corp card back. Click a card to inspect
its upright authorized face and effective text. Close inspection to make a choice.
The run-history panel shows the consequences of every accepted decision.

One short route is **Install Palisade → Trash Brân → Rez Palisade**. The fixed
Runner continues between those choices; Palisade costs three credits and ends
the run. Restart the position to try **Keep Brân**, **Leave unrezzed**, or
**Decline installation**. All decisions, including the fixed Runner's, pass
through the same revision-checked core boundary.

## Engineering checks

Run the canonical input-independent check before publication:

```sh
scripts/check
```

It checks formatting across the full workspace first, then strict Clippy and tests
with both default and `tabletop` features. The Ubuntu CI workflow runs the same
entry point. A focused Cargo command is useful during a repair but does not replace
this final check. Optional driver probes remain available:

```sh
cargo run --locked --bin netrunner-driver -- rez
cargo run --locked --bin netrunner-driver -- unrezzed 2
cargo run --locked --bin netrunner-driver -- jack-out
cargo run --locked --bin netrunner-driver -- rez --save /tmp/bran-check.json --stop-after 2 --view runner
cargo run --locked --bin netrunner-driver -- rez --save /tmp/bran-check.json
```

The driver is an asset-independent engineering probe, not a native no-art mode.
Current Brân regressions cover decline, retain, trash, inward position, paid rez,
unaffordable rez, both jack-out windows and empty HQ through ordinary transitions.
Checkpoints round-trip at every tested decision; the match continues after run
end. The controller/driver checks also cover stale choices, nonmutating legacy
refusal, failed writes and recovery. Current native inspection uses the complete
turns artwork and ordinary table:

```sh
cargo run --locked --bin netrunner-driver -- rez --save /tmp/bran-current.json --stop-after 1
cargo run --locked --features tabletop --bin netrunner-tabletop -- --turns --seat corp --save /tmp/bran-current.json --assets /absolute/path/to/turns-assets
```

At the suspended install, choose Trash Brân's installed card, then Finish
installation. After the Runner advances, rez Palisade for 3 credits. The native
buttons carry the ordinary card-target labels. The two ignored tests require
the private prepared collection; the rendering check also needs Vulkan and a
fresh output path:

```sh
NETRUNNER_ART_ROOT=/absolute/path/to/bran-assets cargo test --locked \
  --test assets real_collection_is_relocatable_and_corruption_is_refused -- --ignored --exact
NETRUNNER_ART_ROOT=/absolute/path/to/turns-assets \
NETRUNNER_RENDER_CAPTURE=/absolute/path/to/bran-render.png \
WGPU_BACKEND=vulkan cargo test --locked --features tabletop --lib \
  tabletop::tests::actual_renderer_uploads_required_images_and_refuses_changed_pixels \
  -- --ignored --exact
```

The latter renders the actual table initially, in card inspection, with the
installed unrezzed Corp back, and after run completion. It checks upload admission and changes one
in-memory required image to verify immediate refusal.
It opens no desktop window. This checks rendering and the application boundary;
physical mouse input, readability in use and owner acceptance require the native
inspection in issue #1.

### Visual preflight before a native handoff

For an artwork or tabletop presentation change, open the prepared affected faces
and the existing renderer captures before handing the candidate to the owner.
With `bran-render.png` as the requested capture, the test also writes
`bran-render.inspection.png`, `bran-render.unrezzed.png` and `bran-render.ended.png`.
Review them together at comparable framing:

- Fronts have complete left/right borders, no neighbouring printing or page strip,
  the intended identity/orientation and readable detail; inspect The Syndicate's
  crop edges explicitly when its recipe changes.
- Table lighting preserves card and token detail, and the installed unrezzed card
  displays the selected Corp back with the intended orientation and proportions.
- Inspection uses the upright authorized face and rounded corners consistent with
  the 3D card; also inspect the modal's text and Close control.

Give reviewers the actual images, candidate/recipe revisions and the corresponding
Arkham implementation from the inspected checkout: `src/tabletop_ui.rs` for
inspection and `src/tabletop_pieces/card_shape.rs` for the shared card geometry
adapter. Trace both consumers to the pinned `unfolded-tabletop` source when a shared
mechanism changes. Record observations and unavailable comparisons in the issue
handoff. Successful hashes, image uploads or source review establish their own
checks; they do not establish visual quality, physical input or owner acceptance.
Use the already agreed native-inspection allowance for those remaining claims.

## Rules basis and shared source

The independent oracle is [CR 26.03](https://access.nullsignal.games/CompRules/Null_Signal_Games_Netrunner_Comprehensive_Rules_v26.03.pdf),
especially 6.2.6 (run position), 6.6 (movement), and 8.5.6b/8.5.10/8.5.13a
(installation, encounter termination and hidden information), together with
[Brân's official rulings](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/rulings/bran_1_0.json)
and card text at that same card-data revision. The source hub and card-data HEAD
were refreshed on 2026-11-07. Printed artwork is presentation, not rules authority.
The hidden-card comparison fixture changes only the hidden face before it needs
execution; it does not implement additional playable cards.

At publication of this entry, both consumers' Cargo manifests and lockfiles pinned
`unfolded-tabletop` and its unchanged `wgpu-hal` patch to
`8f709f71d34e7193ca096f71d7810d1a494b43a8`. The save/resume increment in
[#10](https://github.com/joeloverbeck/netrunner-unfolded/issues/10) advances both
pins to `987b77a38653f6e23cd707171e2c696c3ca59f54`, consuming shared locked,
recoverable file replacement after the shared and Arkham migration merge/pull
gate. Netrunner owns save compatibility, complete match payloads, knowledge and
candidate-before-install policy. Historical sibling revisions are distinguished in the
[2026-11-14 audit](research/evidence.md#next-match-source-audit-2026-11-14).
Arkham's consumer keeps its rules, layout, saves, asset-specific materials and
native health policy. The shared code owns geometry, solid token assembly,
verified reads/crops/publication and decoded loading primitives. The original
Arkham source is `83bd1264b7962d844cfa2417d7f50be642575c9a`.
Reverting the two consumer commits together restores the prior application
contracts; Arkham's prior commit retains its original local implementations.
Issue #1 records exact candidate revisions, check results, publication and
the owner's native acceptance of this bounded entry.

## Rendered agent input

Add `--agent-seat` to the tabletop command to use the same admitted table and native
button handling without opening a desktop window. The selected save has the same
read/write behavior as native play. For an engineering continuation, retain
`--turns --seat corp|runner --save PATH`; use a task-owned copy for diagnosis.

Without `--socket`, standard input and output carry one JSON request/response per
line. `--socket /absolute/path.sock` holds the admitted table across connections
and exits after 30 idle minutes or `{"op":"quit"}`. `{"op":"observe"}` reports
the lawful player observation, `table.revision`, and shown controls. Activate a
reported choice with `{"op":"activate","revision":13,"choice":"FinishInstall"}`;
use its exact `choice` string, including compact JSON for actions with parameters.
Nonchoice controls use `component`, such as `history` or `close-inspection`.
`{"op":"settle"}` waits for native opponent responses, and
`{"op":"capture","path":"/tmp/table.png"}` captures the rendered table.
Clipped controls and stale revisions are refused. This adapter exposes pointer
input; it does not add keyboard navigation or scrolling commands.

The gated `rendered_installation_controls_commit_at_supported_size` test uses
`NETRUNNER_ART_ROOT`, a revision-13 Corp installation copy named by
`NETRUNNER_SEAT_INSTALLATION_SAVE`, and `NETRUNNER_SEAT_CAPTURE_DIR`. Run it in
separate processes with `NETRUNNER_SEAT_TEST_WIDTH=1600` and `1280` and
`cargo test --locked --features tabletop --lib rendered_installation_controls_commit_at_supported_size -- --ignored --nocapture`.
It checks actual hover/press/release, stale and modal refusals, unchanged source
bytes, installation and the six-credit rez. This is automated input evidence;
native usability and owner acceptance remain separate.

The seat consumes `unfolded-tabletop` at the revision `Cargo.toml` pins. Its UI
target helper arrived with
[shared PR #16](https://github.com/joeloverbeck/unfolded-tabletop/pull/16) and
[Arkham migration #240](https://github.com/joeloverbeck/arkham-unfolded/pull/240).
The shared library owns transport, input guards, capture, and the start's order and
refusals ([shared issue #32](https://github.com/joeloverbeck/unfolded-tabletop/issues/32)).
Netrunner owns lawful observation, naming, its admission conditions and the words
for one that runs out of time, settling and its two-camera assembly. Building or running this app requires no
sibling checkout. The existing Arkham `tools seat` client is optional development
tooling; raw JSON lines and the socket protocol are sufficient.

## Bounded match diagnostics

[Issue #23](https://github.com/joeloverbeck/netrunner-unfolded/issues/23) adds an
asset-independent diagnostic consumer of the same introductory Match. Build with
`cargo build --locked --bin netrunner-diagnostics`, then use that executable:

```text
netrunner-diagnostics record 42 /absolute/fresh/primary
netrunner-diagnostics reproduce /absolute/fresh/primary/private.jsonl
netrunner-diagnostics source-case /absolute/fresh/hq-witness
netrunner-diagnostics reproduce /absolute/fresh/hq-witness/private.jsonl
netrunner-diagnostics pilot /absolute/fresh/pilot
```

Use a task-owned external directory; each destination must be new. These commands
do not load or replace native saves. Freeze the executable before retaining
evidence: reproduction requires its exact SHA-256, the supported content and
policy identities, the declared initial seed/case and every intermediate state
digest. `identity` prints the executable hash. Retain the reviewed source commit,
completed canonical check, build command, binary and record hashes together in
the issue's build handoff. That external handoff owns the source-to-executable
association; a source label is not executable proof. Rebuilding after any change
can require the old frozen executable. No cross-version replay is promised.

`private.jsonl` contains the full initial checkpoint and choices. It is solely
for reproduction. `corp.jsonl` and `runner.jsonl` separately contain that role's
contemporaneous observations (including legal alternatives and lawful history),
selected actions, policy latency and finite tactical assessments. Neither the
private record nor later outcomes feed the role policies or those assessments.
Keep these role streams separate: neither is a public match transcript. Files are
created with owner-only permissions. A missing final record means interruption;
reproduction refuses incomplete or changed records and leaves them untouched.

The pilot declares its plan before execution: Corp-v0 versus itself against
Runner-v0 on development seed 3 and diagnostic held-out seed 7039; Runner-v0
versus itself against Corp-v0 on development seed 89 and diagnostic held-out seed
9151. This unchanged-policy comparison validates pairing and reproduction, not
strength. Existing setup seeds 3, 42, 89 and 117 remain development cases.
Seeds 104729, 130363, 155921 and 196613 are reserved for later role candidates and
must remain unused until their final evaluation. Once an inspected result guides
an edit, it is development evidence for that edit, regardless of its old label.

The cap is eight games, 4,000 validated decisions per game, 120 seconds across the
batch, and 256 MiB of output. `plan.json` fixes roles, opponent, splits, seeds and
limits; `report.json` retains every planned arm, including censored, failed and
unstarted games. Completion, legality failures, limited mistake assessments,
unassessed decisions and latency remain separate. Privacy comes from the paired
hidden-state tests, not from wins. The CLI also accepts
`compare PLAN.json FRESH-DIR` for a declared bounded plan using admitted policies.
Policy RNG/memory are absent in v0; gameplay chance stays in the real engine.

The only automatic tactical assessments are a known immediate six-point
score/steal, an optional Hub draw with fewer than two R&D cards, and continuing
into exposed lethal Karunā damage. Other decisions remain **unassessed**, even in
a win. The source witness deliberately stops at Manegarm with Corp 0 credits,
Runner 4 pool credits/3 clicks and 3 hosted Overclock credits. This is a selected
source-derived suspension, not a discovered production failure. Ordinary play,
experienced-casual strength and owner enjoyment require their separate evidence.

The policy source in `src/policies/frozen.rs` preserves the incumbent from
`db313eed`; new Corp/Runner policies must remain separately callable and leave
that incumbent intact. Match diagnostics do not add a second simulator, training,
automatic promotion or a user-facing replay mode.
