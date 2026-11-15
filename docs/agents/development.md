# Bounded development commands

Build `cargo build --locked -p netrunner-dev`; examples below use
`target/debug/netrunner-dev`. This development package directly consumes the
pinned `unfolded-dev-tools` crate; no Arkham checkout or helper executable is
needed. The shared implementation was extracted from Arkham
`d0b57009ba037ca8756a4f2756dcb550d8634773` and is pinned in `tools/Cargo.toml`.
Shared PR #28 and Arkham PR #250 were merged and both main branches pulled before
this adoption (issue #37). The game runtime package does not depend on the
development package. Canonical workspace checks cover both.

## Read and check

```sh
target/debug/netrunner-dev search src/tabletop.rs --find 'NETRUNNER_SEAT'
target/debug/netrunner-dev read src/tabletop.rs --start 3935 --lines 45
target/debug/netrunner-dev read-batch local/verification/requests.json
target/debug/netrunner-dev record local/verification/check-CANDIDATE -- scripts/check
target/debug/netrunner-dev summary local/verification/check-CANDIDATE/check.log
```

Batch requests are a JSON array, for example
`[{"path":"src/tabletop.rs","start":3935,"lines":45}]`. All readers share a
fixed 6,000-byte budget, including metadata. Follow `NEXT` with its hash and
cursor until `END`; omit `--max-bytes` from the helper's continuation because the
wrapper supplies it. A changed source fails instead of mixing revisions. A line
too long for the budget fails explicitly; inspect selected fields separately.
Use `rg` on selected paths for discovery; do not dump whole files or whole GitHub
responses. Select relevant JSON fields, and retain large responses on disk.

`record` requires a fresh output directory. Complete stdout/stderr stay in
`check.log`, with command, cwd, source identity and terminal status in `run.json`.
It succeeds only when the child succeeds and that record is `finished`, exit 0,
with unchanged source. A lost caller session is a reason to inspect the record,
not repeat the check. Running, interrupted, invalidated or missing records are
not passing evidence. Inspect failures with `summary`, then read selected log
ranges. Do not rerun unchanged passing work without a new concern.

## Native preflight

```sh
target/debug/netrunner-dev preflight
# Or supply both paths explicitly:
target/debug/netrunner-dev preflight /path/to/turns-assets local/verification/native-CANDIDATE
```

The default artwork is `NETRUNNER_ART_ROOT`, otherwise
`$XDG_DATA_HOME/netrunner-unfolded/turns-assets` (falling back to
`~/.local/share`). Missing or changed artwork fails admission. A Vulkan-capable
local environment is required; this is not added to public CI.

The command creates a fresh seeded ordinary Runner save in a new evidence
directory; it never selects the personal match save. Through the existing
production setup and opponent it reaches an ordinary Runner action phase, then
runs the existing growing-hand/both-seat clearance, rendered action/count update,
and inspection/artwork-refusal tests sequentially. It rejects a zero-test filter
or missing capture. Each stage keeps its log and completion record. `README.txt`
indexes the evidence; `PASS` appears only when all stages complete. Inspect the
PNG captures when making a visual claim. This preflight covers those specific
regressions; it does not establish complete-game correctness, physical input
usability, or owner acceptance.

## Custody and publication

Use persistent worktrees under `~/.cache/agent-tmp` and ignored
`local/verification/<task>` evidence, with one issue-linked handoff index naming
the revision, commands, outcomes and remaining work. Keep return-use launchers
there too; reserve `/tmp` for disposable scratch and sockets. Reuse a suitable
Cargo target directory for compatible worktrees instead of multiplying full
builds. For storage diagnosis use Arkham's existing `target-prune --help` and
dry-run inventory; deletion is a separate authorized action, never preflight.

Keep independent source review, but let the coordinator perform routine
integration and publication. An agent whose only job is to run merge commands
adds no independent review. Retain a completed canonical `scripts/check` for the
candidate. Read back the PR head, base, body and changed-file scope; changes to
that candidate invalidate the corresponding review/check claims.

Use one `gh run watch RUN --compact --exit-status --interval 30` for the required
CI run, retaining its output on disk. Verify the terminal run conclusion and
`headSha` against the exact PR head; a watcher exit alone is not proof. Report
state transitions where host instructions permit, without repeated CI narration.
The host's progress-update frequency is outside repository control.

After authorized review and exact-head required checks, use
`gh pr merge NUMBER --merge --match-head-commit HEAD_SHA`. Read back merged state,
switch to main, run `git pull --ff-only origin main`, and verify the merge is in
both local and remote main and the working tree is clean. Do not use Arkham's
repository-specific `merge-reviewed` against Netrunner. Preserve explicit owner
acceptance gates; merge authority does not close unrelated gameplay acceptance.
