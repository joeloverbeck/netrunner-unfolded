# Issue tracker: GitHub

Issues and specifications live in [joeloverbeck/netrunner-unfolded](https://github.com/joeloverbeck/netrunner-unfolded/issues). Use the `gh` CLI from this clone.

GitHub Issues and PRs are the sole active work-control plane. Local documents may support an issue; they do not form a second queue. Publishing a ticket or specification does not activate implementation. Follow the repository constitution and the accepted issue for scope, effort limits, acceptance, and publication. Read the current preparation and activation boundary in `docs/foundation/01-mission-and-play.md`.

## Conventions

Before the first write of a session, verify that `gh repo view --json nameWithOwner` resolves to `joeloverbeck/netrunner-unfolded`. Use `--repo joeloverbeck/netrunner-unfolded` when running outside this clone. File issues only in this repository, not in the upstream skills repository.

- **Create:** `gh issue create --title "..." --body-file <file>`.
- **Read:** `gh issue view <number> --json number,title,body,state,labels,comments`.
- **List:** `gh issue list --state open --json number,title,body,labels,assignees`, with appropriate label or state filters.
- **Parent/child:** create with `gh issue create --parent <parent> ...`, or link an existing child with `gh issue edit <parent> --add-sub-issue <child>`. If native sub-issues are unavailable, put `Part of #<parent>` in the child body and link the child in the parent's task list.
- **Blocking:** use native dependencies: `gh issue create --blocked-by <number> ...` or `gh issue edit <child> --add-blocked-by <blocker>`. If unavailable, put `Blocked by: #<number>` at the top of the child body.
- **Check blockers:** read `gh issue view <number> --json blockedBy` and inspect blocker states; also inspect any fallback `Blocked by` references. Closed blockers clear only the dependency condition, not work authorization.
- **Comment:** `gh issue comment <number> --body-file <file>`.
- **Labels:** `gh issue edit <number> --add-label <label>` or `--remove-label <label>`; use the mapping and readiness rules in `docs/agents/triage-labels.md`.
- **Close:** post the explanation as a separate comment, then run `gh issue close <number>`. On an already-closed issue, `gh issue close --comment` can omit the comment while returning success.

For multiline bodies and comments, write the exact text to a temporary file and use `--body-file`. Read back the changed issue, comment, or labels after a write.

GitHub shares issue and PR numbers. Resolve a bare number with `gh pr view <number>`, falling back to `gh issue view <number>` when it is not a PR; an access or network failure does not establish the object type.

## Bounded diagnostics and handoff

Keep one output budget across a batch of reads, initially about 6,000 bytes. Search
explicit files/ranges first, then read the relevant passages through completion;
truncation is a reason to narrow or page the read. When using the Arkham tooling
checkout during coordinated work, reuse its `search-context`, `read-context-batch`
and `check-summary` commands as documented in its tracker and README. Follow each
reader's `NEXT` through `END`; inspect a failed check's retained log with
`check-summary` before expanding selected ranges. Keep the full log on disk.
This reuses optional development tools and creates no ordinary build dependency.

Before publication, retain a completed `scripts/check` result for the exact
candidate. A focused test or root-package format check cannot stand in for the
full-workspace entry point. For tabletop or asset changes, attach the
[visual preflight](../bran-run.md#visual-preflight-before-a-native-handoff) images
and reference implementation to review and native handoff evidence.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Skill instructions

When a skill says "publish to the issue tracker", create a GitHub issue within the authorized scope. When it says "fetch the relevant ticket", read that issue and its comments.

## Wayfinding operations

These conventions apply when wayfinding is explicitly selected; setup creates no map, tickets, or additional labels.

- **Map:** one issue labelled `wayfinder:map`, containing Notes, Decisions-so-far, and Fog.
- **Child ticket:** a sub-issue of the map, using the parent/child convention above, labelled `wayfinder:<type>` (`research`, `prototype`, `grilling`, or `task`). Decision tickets use these labels rather than triage readiness labels.
- **Frontier:** inspect the map's sub-issues or fallback task list in map order. Consider open, unassigned children with no open blockers, subject to the accepted scope and authorization.
- **Claim:** `gh issue edit <number> --add-assignee @me`.
- **Resolve:** comment with the answer, close the ticket, and append a concise result and link to the map's Decisions-so-far.
