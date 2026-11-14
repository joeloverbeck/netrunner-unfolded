# netrunner-unfolded

Foundation for a personal Rust/Bevy implementation of Null Signal Games' Netrunner, with a physical 3D table and computer opponents for both roles.

This repository starts from the research and documentation foundation prepared on **2026-11-05**, with owner decisions updated on **2026-11-07**. Jon judged **The Gathering** in [arkham-unfolded](https://github.com/joeloverbeck/arkham-unfolded) complete enough to begin Netrunner. The accepted first implementation is the bounded Corp-side Brân run described below; full games and trained bots remain unimplemented.

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

Use [Brân run: preparation, launch and checks](docs/bran-run.md) to build the Rust application, prepare the required external artwork and play the bounded position. Run `scripts/check` before publication for full-workspace formatting, strict Clippy and tests with both default and tabletop features. These checks need no private artwork or desktop; the guide separates asset/GPU checks and visual inspection. Card PDFs and prepared imagery remain outside Git. `AGENTS.md` is tracked as a symlink to `CLAUDE.md`.

The first specification is [issue #1: Complete a Corp-side Brân run through the match system](https://github.com/joeloverbeck/netrunner-unfolded/issues/1). It records candidate revisions, checks, publication and the owner's native acceptance; it closed on **2026-11-08**. That acceptance covers this bounded entry, not complete matches or bots. Automated checks and owner acceptance remain distinct evidence.

The [Arkham-first workflow](docs/foundation/05-shared-code-and-development.md) governs future reuse. Shared code belongs in `unfolded-tabletop` when a concrete extraction serves both applications; the sibling Arkham checkout is for investigation, not an ordinary build dependency.

Original project code uses the [MIT license](LICENSE), matching Arkham Unfolded. That choice grants no license to Netrunner card text, artwork, symbols, or third-party implementations; applicable notices remain separate. This is an independent fan project without an official endorsement claim.

The current introductory-match integration also provides bounded [central access and agenda checkpoints](docs/central-access.md), including the source-defined Jailbreak winning steal. Full-match admission and owner acceptance remain separate from this engineering evidence.
