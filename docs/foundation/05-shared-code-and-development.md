# Shared code and development

## Ongoing sibling-reuse workflow

**Investigate Arkham first, and inspect Mudsong as well, throughout Netrunner's development.** Before implementing required behavior, inspect the current [arkham-unfolded](https://github.com/joeloverbeck/arkham-unfolded), private `mudsong`, and `unfolded-tabletop` implementations, tests, and applicable instructions. This includes development, verification, asset-preparation, and native-operation tooling. When either game already implements a required common mechanism, extract and consume it; do not copy it or independently reimplement it here. This obligation does not expire after the starter-deck milestone. Once extracted, the public `unfolded-tabletop` repository owns the shared implementation; consuming applications contribute there rather than maintaining divergent copies. Game-specific dispositions still require the semantic justification below.

Use an explicitly configured Arkham checkout if supplied, otherwise look for `../arkham-unfolded`. Verify its repository identity and inspect local changes before editing. If absent, clone the public repository into that sibling location; never replace an unrelated directory or discard existing work. If the environment cannot provide the checkout, resolve that prerequisite rather than guessing from stale snippets. Record the inspected commit in the active issue/PR. Normal builds must work without this sibling checkout.

Inspect `../mudsong` and `../unfolded-tabletop` with the same identity and local-change checks. Mudsong is private: preserve its content custody and do not infer permission to publish its game-specific material in the shared public library. Distinguish local checkout revisions, existing remote-tracking refs, and freshly verified remote revisions. Research checkouts remain optional development inputs, never ordinary build dependencies.

For each candidate, choose from evidence:

- **Already shared:** consume the pinned shared version or improve it with affected real consumers exercised.
- **Reusable in a sibling:** isolate a concrete common mechanism, keep game policies in local adapters, and migrate its real consumers through a coordinated change. Verify affected sibling behavior before continuing the dependent Netrunner feature; a locally copied implementation is not an interim substitute for this migration.
- **Similar but game-specific:** retain local implementations; record why forced sharing would distort either application.
- **Absent:** implement the smallest justified Netrunner path. Anticipated future sibling demand is not a second consumer.

Searches and a short disposition belong in the existing work artifact. No universal extraction backlog or separate reuse-report system is needed. Do not found an empty shared framework before its first concrete extraction.

Jon's **2026-11-14 sequencing requirement** applies before Netrunner starts using a required shared feature: complete the coordinated extraction and affected sibling migrations, verify them, merge the required Arkham and Mudsong PRs, and pull their `main` branches. Record the merged revisions and local-main readback in the linked work artifacts. Only then begin dependent Netrunner implementation against an immutable shared revision. An already-shared mechanism needs evidence of its completed migrations rather than another extraction. This order is a constraint on later authorized work, not publication approval for an unspecified patch. Do not add a game feature to a sibling merely to manufacture a consumer.

The [2026-11-14 source audit](../research/evidence.md#next-match-source-audit-2026-11-14) distinguishes already-shared mechanisms from further extraction candidates. The instruction and preparation consulted `PURP-1`–`PURP-6`, `WORK-1`–`WORK-6`, `AUTH-2`–`AUTH-3`, `EVID-3`–`EVID-8`, `SEM-1`–`SEM-2`, `REP-1`–`REP-3`, `BOUND-1`–`BOUND-5`, `INT-1`–`INT-4`, `AGENT-1`–`AGENT-5`, `OPS-1`–`OPS-7`, `GOV-1`–`GOV-3`, and `ADOPT-2`–`ADOPT-3`. The existing waivers below retain their exact scopes and limits.

## Inspected candidates

The 2026-11-07 static refresh inspected clean Arkham commit `0903b6454a8f92b16a91e93b052704bb23fe95d8`. [Evidence](../research/evidence.md#arkham-refresh-2026-11-07) identifies existing card meshes, solid textured tokens, asset integrity and renderer admission, and Rust tooling, including relevant tests and migration risks. These were candidates at that inspection; the later bounded extraction and consumer acceptance are recorded in [issue #1](https://github.com/joeloverbeck/netrunner-unfolded/issues/1) and its [implementation guide](../bran-run.md#rules-basis-and-shared-source). Refresh affected source before new implementation; preserve game-specific artwork/shape decisions, layout, action handlers, save admission, and native-operation policies in consumer adapters.

Game rules, action/timing models, deck definitions, card effects, bot policies, and game save semantics remain in their application. The game-specific disposition above requires a concrete semantic difference; it is not an escape hatch for duplicating a common mechanism or tooling. Do not invent a universal tabletop rules engine or share domain enums just because both games have cards and resources. Shared code must not import either game's domain. This boundary applies `BOUND-2`, `BOUND-3`, `INT-1`, and `AGENT-1`; coordinated changes preserve each repository's authority.

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

### Third consumer

The exception above covers code used by both `arkham-unfolded` and `netrunner-unfolded`, and a third consumer ends its authority for the surface it touches. On **2026-11-11** Jon accepted a further scoped waiver under `ADOPT-2` so that `mudsong`, a private repository of his, can be that consumer. Its authoritative text is that repository's `docs/foundation/04-engineering-and-shared-code.md`. This is its record here, carried by the first extraction made under it. It edits neither the constitution nor the exception above, which still governs code shared by Arkham and Netrunner alone.

| Field | Accepted boundary |
| --- | --- |
| Exact rule | `EVID-6`: three materially different real cases and an independent holdout before cross-product adoption |
| Scope | Concrete mechanisms, not specific to any game, owned in `unfolded-tabletop` and really used by `mudsong` and by at least one of `arkham-unfolded` and `netrunner-unfolded` |
| Reason | These three are the only real products. Duplicating mechanisms or inventing nominal consumers would defeat the intended reuse |
| Accepted risk | Two consumers can miss an incompatible assumption; a later case may force an interface change or a reversal |
| Required evidence | A real path in each consuming application, each repository's own checks, game policy preserved in adapters, and a coordinated rollback |
| Appetite | Two active hours per extraction attempt, then reassess. No extension into a framework project |
| Ends | For a given mechanism: a consumer outside these three games, publication to crates.io or a promise of general reuse, or loss of its second real consumer. There is no milestone or date expiry. After that, further sharing needs the normal evidence or a new waiver |

That first extraction moved Arkham's seeded generator and shuffle into `unfolded-tabletop` for Arkham and `mudsong`. Netrunner was inspected at `dc5bef3` and holds no seeded generator or shuffle, so nothing here was migrated and its pin is unchanged. When Netrunner first needs randomness, the shared generator is the **Already shared** case above.

## Integration and activation

Use immutable Git revisions initially, or actual released package versions when available, with lockfiles. Local Cargo overrides can aid a coordinated experiment; ordinary build instructions and CI cannot depend on uncommitted siblings or moving branches. Keep consumer adapters at the consumer. Record producer revision, affected consumer changes/checks, and rollback in the coordinated PRs. A green library test is not evidence that an application's actual path survived migration.

Arkham currently pins Rust 1.97.1 and Bevy 0.20.0. Choose a compatible baseline after refreshing it. Its narrowly targeted NVIDIA/Vulkan `wgpu-hal` patch needs particular care: Cargo applies patches at the workspace root, not transitively from dependencies. Verify whether both applications still need the workaround on Jon's machine and preserve provenance/licenses; do not assume extracting a rendering library transfers the patch.

## First specification boundary

This section preserves the **2026-11-07** design and initial limits; issue #1 records the subsequent implementation, repairs, acceptance and publication. Jon selected the bounded playable run position subject to the [single-match-path decision](../adr/0001-one-match-path.md). The spec required play as Corp through the end of a preset run, choosing whether Brân installs replacement ICE and trashes itself, then observing and responding to the resulting run progression. Use fixed legal Runner responses, real card art and textured 3D tokens. This first product-bearing increment establishes neither complete games nor opponent strength (`EVID-3`, `EVID-4`). The [source-derived worked position](../research/evidence.md#first-run-source-basis-2026-11-07) supplies the starting proposal and independent expected branches.

Use one game boundary for native choices and headless checks: current player observation and legal decision in, validated transition and observable consequences out. Check source-derived timing, normal rez payment, hidden information, stale/invalid choice refusal, and completion of the run through that boundary. Native inspection must additionally establish readable cards/tokens, usable choices, and understandable consequences. Asset failure checks must establish refusal before play or persistence; files present on disk are insufficient. Jon confirmed this testing seam for `to-spec` on 2026-11-07; acceptance of the seam is not completed verification (`REP-2`, `OPS-1`).

Extract only existing common mechanisms needed by this slice, with both consumers using the pinned result and the affected Arkham path verified. Include actual textured cards/tokens and artwork admission in Arkham checks, plus affected interaction or persistence behavior. Preserve a coordinated rollback and independent builds; a zero-test filter or a green shared-library suite cannot establish consumer behavior. Full setup, strategic bots, save/resume, undo, the remaining cards and generic framework work are outside this first task, while remaining part of the full milestone where specified.

Accepted limits for the initial implementation attempt: stop after **two active hours without an inspectable acceptance-bearing result**, with at most **two native inspection sittings of 15 minutes each in total across both applications**. Rejected and inconclusive sittings count. Record the evidence and reassess when a limit is reached; the shared extraction does not receive a fresh allowance on switching repositories. Existing `OPS-4` and the scoped `EVID-6` exception also apply.

Jon confirmed the final synthesis and testing boundary on **2026-11-07**. The resulting specification, [GitHub issue #1: Complete a Corp-side Brân run through the match system](https://github.com/joeloverbeck/netrunner-unfolded/issues/1), was published with `ready-for-agent`. That issue owns work status, implementation evidence and acceptance; this section records its accepted design input rather than a second queue. The label recorded bounded readiness, not implementation activation, unattended authority or production publication. No implementation, shared repository or migration was created by specification preparation itself; those followed under the later issue authorizations.
