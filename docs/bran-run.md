# Brân at HQ

This entry implements the accepted bounded position in [issue #1](https://github.com/joeloverbeck/netrunner-unfolded/issues/1).
You play the Corp against a Runner that continues whenever allowed. Decline the
installation, install Palisade and keep Brân, or install and trash Brân before
deciding whether to pay three credits to rez Palisade. The run finishes; this
entry does not continue into unsupported turns or declare a match winner.

## Prepare and launch

Use Rust 1.97.1, Ubuntu's Vulkan/X11 libraries, Poppler (`pdftoppm`), and the
recorded DejaVu Sans font from `fonts-dejavu-core`. The selected PDF is the
[System Gateway English A4 3× sheet](https://access.nullsignal.games/Gateway/English/English/SystemGatewayEnglish-A4%20Printable%20Sheets%203x.pdf).
Keep it and the supplied Android Netrunner resource collection outside Git.
The exact source identities, crop coordinates and resulting image hashes are
in [`resources/bran-assets.json`](../resources/bran-assets.json).

```sh
cargo run --locked --bin netrunner-assets -- prepare \
  /absolute/path/to/SystemGatewayEnglish-A4.pdf \
  '/absolute/path/to/Android Netrunner Resources' \
  /absolute/path/to/corp-back.jpg \
  /absolute/path/to/bran-assets
cargo run --locked --bin netrunner-assets -- check /absolute/path/to/bran-assets
cargo run --locked --features tabletop --bin netrunner-tabletop -- \
  --assets /absolute/path/to/bran-assets
```

The default runtime collection is
`${XDG_DATA_HOME:-$HOME/.local/share}/netrunner-unfolded/bran-assets`.
Preparation preserves the original inputs and any existing output directory.
An identical complete collection is reused; incomplete or different output is
refused. Choose a fresh folder when the recipe changes. No credentials, network
service or LLM is used during gameplay. There is no save file in this entry.

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
```

The driver is an asset-independent engineering probe, not a native no-art mode.
Tests cover all finite branches in the issue, both players' information rights,
stale/out-of-turn/unavailable choices, admission refusal, restart revisions, native
button handlers, inspection and token materials. The two ignored tests require
the private prepared collection; the rendering check also needs Vulkan and a
fresh output path:

```sh
NETRUNNER_ART_ROOT=/absolute/path/to/bran-assets cargo test --locked \
  --test assets real_collection_is_relocatable_and_corruption_is_refused -- --ignored --exact
NETRUNNER_ART_ROOT=/absolute/path/to/bran-assets \
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

Both Cargo manifests and lockfiles pin `unfolded-tabletop` and its unchanged
`wgpu-hal` patch to `8f709f71d34e7193ca096f71d7810d1a494b43a8`.
Arkham's consumer keeps its rules, layout, saves, asset-specific materials and
native health policy. The shared code owns geometry, solid token assembly,
verified reads/crops/publication and decoded loading primitives. The original
Arkham source is `83bd1264b7962d844cfa2417d7f50be642575c9a`.
Reverting the two consumer commits together restores the prior application
contracts; Arkham's prior commit retains its original local implementations.
Issue #1 records exact candidate revisions, check results, publication and
the owner's native acceptance of this bounded entry.
