# Shared code and development

## Ongoing Arkham-first workflow

**Arkham remains the main feature driver throughout Netrunner's development.** Before implementing non-game-specific functionality, inspect the current [arkham-unfolded](https://github.com/joeloverbeck/arkham-unfolded) implementation, tests, and applicable instructions. This obligation does not expire after the starter-deck milestone. Once extracted, `unfolded-tabletop` owns the shared implementation; both applications contribute there rather than maintaining divergent copies.

Use an explicitly configured Arkham checkout if supplied, otherwise look for `../arkham-unfolded`. Verify its repository identity and inspect local changes before editing. If absent, clone the public repository into that sibling location; never replace an unrelated directory or discard existing work. If the environment cannot provide the checkout, resolve that prerequisite rather than guessing from stale snippets. Record the inspected commit in the active issue/PR. Normal builds must work without this sibling checkout.

For each candidate, choose from evidence:

- **Already shared:** consume the pinned shared version or improve it with both consumers exercised.
- **Reusable in Arkham:** isolate a concrete common mechanism, keep game policies in local adapters, and migrate both real consumers through a coordinated change before continuing the dependent Netrunner feature.
- **Similar but game-specific:** retain local implementations; record why forced sharing would distort either application.
- **Absent:** implement the smallest justified Netrunner path. Anticipated future Arkham demand is not a second consumer.

Searches and a short disposition belong in the existing work artifact. No universal extraction backlog or separate reuse-report system is needed. Do not found an empty shared framework before its first concrete extraction.

## Inspected candidates

Static audit basis: Arkham commit `d17075e873d0721f65840f9fdc338755e8a0de52`. [Evidence](../research/evidence.md#arkham-audit) links exact files and limitations. Refresh before implementation; these are candidates, not accepted migrations.

| Mechanism | Plausible extraction | Keep local / verify |
| --- | --- | --- |
| Card geometry | Parametric face/body/outline in `card_shape.rs` | Art selection, ownership, game layout; both native renderings |
| Save replacement | `record_replacement.rs` IO lifecycle | Admission, save schema, recovery policy; no multi-file transaction claim |
| Camera | Orbit/focus/input mechanics | Arkham investigator/area presets versus Netrunner seats/servers |
| Counters/pieces | Geometry, picking, animation after dual demand | Clues, chaos bag, harm symbols, tags, hosted-credit meaning |
| Asset handling | Checked paths, hashes, reusable crop/cache operations | Arkham archive/font extraction versus Netrunner PDF adapter |
| UI/pacing/capture | Narrow modal-input, frame, or inspection mechanisms when needed | Game actions, layout policy, native acceptance evidence |

Game rules, action/timing models, deck definitions, card effects, bot policies, and game save semantics remain in their application. Do not invent a universal tabletop rules engine or share domain enums just because both games have cards and resources. Shared code must not import either game's domain.

## Accepted EVID-6 exception

Decision owner: Jon; accepted **2026-11-05**, including his clarification that sharing continues beyond the first milestone. This records the scoped waiver under `ADOPT-2`; it does not edit the constitution.

| Field | Accepted boundary |
| --- | --- |
| Exact rule | `EVID-6` requirement for three materially different real cases plus an independent holdout before cross-product adoption |
| Scope | Concrete non-game-specific extractions used by **both** `arkham-unfolded` and `netrunner-unfolded`, owned in `unfolded-tabletop` |
| Reason | These are the only real products; duplicating useful mechanisms or inventing nominal extra consumers would frustrate the intended reuse |
| Accepted risk | Two consumers can miss incompatible assumptions; later cases may require interface changes or reversal |
| Required evidence | An actual path in each application, consumer-specific verification, preserved game policies, and a coordinated rollback path |
| Appetite | At most two active hours per extraction attempt before reassessment; no automatic extension into a framework project |
| Ending event | For an affected surface, a third consumer, a broader reusable contract, crates.io publication, or loss of its concrete second consumer ends this exception's authority for further adoption/expansion; reassess with the normal evidence or a new explicit waiver |

There is **no starter-milestone expiry**. Maintenance of already accepted behavior remains possible; the exception is not permission to expand indefinitely. It establishes neither general library fitness nor human acceptance of untested migrations. Carry the same scoped decision into the coordinated Arkham/shared-repository work; this document does not itself modify their governing files.

## Integration and activation

Use immutable Git revisions initially, or actual released package versions when available, with lockfiles. Local Cargo overrides can aid a coordinated experiment; ordinary build instructions and CI cannot depend on uncommitted siblings or moving branches. Keep consumer adapters at the consumer. Record producer revision, both consumer changes/checks, and rollback in the coordinated PRs. A green library test is not evidence that Arkham's actual native path survived migration.

Arkham currently pins Rust 1.97.1 and Bevy 0.20.0. Choose a compatible baseline after refreshing it. Its narrowly targeted NVIDIA/Vulkan `wgpu-hal` patch needs particular care: Cargo applies patches at the workspace root, not transitively from dependencies. Verify whether both applications still need the workaround on Jon's machine and preserve provenance/licenses; do not assume extracting a rendering library transfers the patch.

Implementation remains gated by finishing The Gathering. When Netrunner work is selected, the proposed first bounded experiment starts from a source-derived mid-run position with scoped cards, hidden information, and a difficult timing branch—for example Brân installing ICE and trashing itself. Use the same core in a minimal native table and headless fixture; inspect an actual player choice and a source-derived result. This bounded position avoids needing full game setup first. Establish only the required geometry reuse, not every candidate above. Begin with a two-hour attempt and report what became inspectable; a credit-counter demo does not establish this premise.

This is a proposed entry slice, not an activated backlog or a replacement for complete games. After it, select the next actual bottleneck from the observed result. GitHub Issues/PRs own work selection and acceptance. Apply the constitution's `WORK`, `OPS`, `INT`, and `PUB` rules directly rather than reproducing them here. No remote repositories, issues, migrations, or publication were performed by preparing these foundations.
