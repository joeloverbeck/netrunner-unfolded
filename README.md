# netrunner-unfolded

Foundation for a personal Rust/Bevy implementation of Null Signal Games' Netrunner, with a physical 3D table and computer opponents for both roles.

This repository starts from the research and documentation foundation prepared on **2026-11-05**. It is not a playable application. Netrunner implementation follows completion of **The Gathering** in [arkham-unfolded](https://github.com/joeloverbeck/arkham-unfolded). Gameplay, trained bots, card extraction, and shared-code migrations are not implemented.

## Read in this order

1. [CLAUDE.md](CLAUDE.md): agent entry point; `AGENTS.md` is its real symlink.
2. [Mission and play contract](docs/foundation/01-mission-and-play.md): accepted scope and what success means.
3. [Rules and content](docs/foundation/02-rules-and-content.md): authoritative sources and exact decks.
4. [Engine and bots](docs/foundation/03-engine-and-bots.md): headless execution, fair observations, and AI development.
5. [Tabletop and assets](docs/foundation/04-tabletop-and-assets.md): interaction, continuity, layout, and external artwork.
6. [Shared code and development](docs/foundation/05-shared-code-and-development.md): ongoing Arkham-first reuse and the accepted exception.

[Research evidence](docs/research/evidence.md) preserves inspected revisions, source links, alternatives, and limits. It is reference material, not an additional work queue. The [repository constitution](docs/principles/principles-for-repos.md) is an unchanged copy of Jon's supplied file.

## Checkout and current status

```sh
git clone https://github.com/joeloverbeck/netrunner-unfolded.git
cd netrunner-unfolded
```

There is no Cargo workspace or build/test command yet. The repository contains no card PDFs, images, executable scaffolding, or third-party source code. `AGENTS.md` is tracked as a symlink to `CLAUDE.md`.

The [Arkham-first workflow](docs/foundation/05-shared-code-and-development.md) governs future reuse. Shared code belongs in `unfolded-tabletop` when a concrete extraction serves both applications; the sibling Arkham checkout is for investigation, not an ordinary build dependency.

Original project code uses the [MIT license](LICENSE), matching Arkham Unfolded. That choice grants no license to Netrunner card text, artwork, symbols, or third-party implementations; applicable notices remain separate. This is an independent fan project without an official endorsement claim.
