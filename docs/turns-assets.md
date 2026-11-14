# Artwork for bounded ordinary-turn positions

[Issues #12](https://github.com/joeloverbeck/netrunner-unfolded/issues/12) and
[#13](https://github.com/joeloverbeck/netrunner-unfolded/issues/13), continued by
[#14](https://github.com/joeloverbeck/netrunner-unfolded/issues/14) and
[#15](https://github.com/joeloverbeck/netrunner-unfolded/issues/15), use the
`ordinary-turns-4` collection in [`resources/turns-assets.json`](../resources/turns-assets.json).
It contains eighteen card faces, four token faces, both opaque card backs and the
existing DejaVu Sans font. It admits only the bounded engineering positions;
this is not a complete introductory-deck collection or a full-match launch path.
The existing [Brân preparation and launch contract](bran-run.md) is unchanged.

Use the same verified System Gateway PDF, supplied Android Netrunner resource
collection, Corp back and Ubuntu Poppler/font prerequisites as Brân, plus the
Runner-back JPEG identified by `runner_back_source` in the new recipe:

```sh
cargo run --locked --bin netrunner-assets -- prepare-turns \
  /absolute/path/to/SystemGatewayEnglish-A4.pdf \
  '/absolute/path/to/Android Netrunner Resources' \
  /absolute/path/to/corp-back.jpg \
  /absolute/path/to/runner-back.jpg \
  /absolute/path/to/turns-assets
cargo run --locked --bin netrunner-assets -- check-turns /absolute/path/to/turns-assets
```

The optional runtime default is
`${XDG_DATA_HOME:-$HOME/.local/share}/netrunner-unfolded/turns-assets`.
Use a fresh destination. Preparation verifies original bytes before transforming
them, verifies and decodes every prepared image, then publishes without replacing
an existing directory. An identical complete collection is reused. A Brân folder,
incomplete folder or changed recipe is refused and preserved. `check` continues
to admit only Brân; `check-turns` admits only the new collection. Native consumers
must also finish image/font loading before creating or restoring game state.

The Runner back is the 1452×2100 JPEG used by the supplied TTS mod, SHA-256
`8044509f2d482773dc2460d866debc85135e96a6ca1798ee5a2db6ef8120d0f6`,
836970 bytes. Use the [existing-source lookup](bran-run.md#locate-an-existing-source-before-declaring-artwork-missing)
with the recipe's Runner URL. Preparation decodes it without resizing. It does
not download artwork. All image payloads and editable documents remain external;
the code license does not license redistribution of their artwork.

The fourteen additional faces were extracted and inspected through PhotoCraft MCP.
Full pages were rendered with Poppler 26.01.0 at 300 DPI, then cropped without
resampling. PhotoCraft's PDF importer accepts at most sixteen pages; the later
Corp pages used unchanged Poppler full-page rasters as editor inputs. Normal Rust
preparation uses the same verified PDF and reviewed rectangles directly. All
coordinates are pixels from the top-left of the one-based PDF page:

| Printing | Card | Page | x | y | Width | Height |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 30006 | Cleaver | 3 | 124 | 2269 | 744 | 1032 |
| 30013 | Docklands Pass | 6 | 124 | 207 | 744 | 1030 |
| 30026 | Unity | 10 | 124 | 1239 | 744 | 1030 |
| 30028 | Jailbreak | 11 | 124 | 207 | 744 | 1030 |
| 30029 | Overclock | 11 | 124 | 1239 | 744 | 1030 |
| 30030 | Sure Gamble | 11 | 124 | 2269 | 744 | 1032 |
| 30040 | Seamless Launch | 14 | 124 | 2269 | 744 | 1032 |
| 30042 | Manegarm Skunkworks | 15 | 124 | 1239 | 744 | 1030 |
| 30064 | Government Subsidy | 21 | 124 | 2269 | 744 | 1032 |
| 30067 | Offworld Office | 22 | 124 | 2269 | 744 | 1032 |
| 30069 | Send a Message | 23 | 124 | 1239 | 744 | 1030 |
| 30070 | Superconducting Hub | 23 | 124 | 2269 | 744 | 1032 |
| 30074 | Whitespace | 25 | 124 | 207 | 744 | 1030 |
| 30075 | Hedge Fund | 25 | 124 | 1239 | 744 | 1030 |

The shorter bounds remove a page strip or neighboring-row line. The original four
Brân face mappings and output hashes remain unchanged. Effective rules text is
owned by the engine's content contract, not by these older printings.

Revision 2 added Overclock and Whitespace; revision 3 added Manegarm Skunkworks;
revision 4 adds Jailbreak, Send a Message and Superconducting Hub.
Prepare into a fresh directory;
the earlier collection is preserved and refused by this revision. This artwork
revision does not migrate or change the separate match-save format.

Blue/red click faces use the left 256×256 half of their 512×256 source atlases.
Advancement uses the left half of `credit-advancement-light.png`; credit keeps the
right half. They remain opaque and are intended for solid textured pieces, with
renderer-owned edges. No flat memory-token face has been established from the
supplied oblique ring image; the collection does not invent that mapping.

The external authoring collection is
`${XDG_DATA_HOME:-$HOME/.local/share}/netrunner-unfolded/authoring/spec-7`:
`cards/<printing>.pcraft`, `tokens/<role>.pcraft`, `backs/runner.pcraft`, their PNG
exports, full-page card rasters and `sources` retain the editor inputs. Open the
native document in PhotoCraft and export its canvas to resume authoring. The
recipe's `reviewed_exports` pins those inspected PNGs; `assets` pins the actual
Rust/Poppler outputs. PNG byte encoders differ, so compare decoded RGBA pixels,
not encoded-file equality. Editors are not build or gameplay dependencies.

Run the private real-input checks after preparing the collection:

```sh
NETRUNNER_TURNS_ART_ROOT=/absolute/path/to/turns-assets \
NETRUNNER_REVIEWED_ART_ROOT=/absolute/path/to/authoring/spec-7 \
cargo test --locked --test assets real_turns_ -- --ignored --nocapture
```

These exercise relocation, refusal of missing/changed new assets, preservation
of an existing save, and pixel equivalence with every new reviewed export. They
do not establish GPU loading, native input, readability in play or owner
acceptance; the parent issue's native checks retain those obligations.
