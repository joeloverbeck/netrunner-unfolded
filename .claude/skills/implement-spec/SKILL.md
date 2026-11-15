---
name: implement-spec
description: "Implement the result of /to-spec and /to-tickets in code."
disable-model-invocation: true
---

You have been provided a spec. This spec should have tickets associated with it, describing how to implement the spec.

The issue tracker should have been provided to you. If not, tell the user to run `/setup-matt-pocock-skills`.

The goal is the entire spec implemented on a single **integration branch**, with every ticket resolved the way the issue tracker closes work.

The tickets are not a list of steps. They are a **task graph** with blocking relationships between them. This means there is always a **frontier** of tickets which are ready to be grabbed.

Use subagents when independent review or parallel ticket work benefits from a
separate context; routine sequential work may stay with the coordinator. Start
every subagent with a fresh context window.

Communication to and from subagents should be sparse. Communicate primarily through **context pointers**: to the spec, tickets, research notes, and previous commits. Don't duplicate information already available via pointers.

**Implementer subagents** should be run in the background where possible for maximum concurrency.

## Steps

1. Read the spec and tickets to understand the task graph.

2. Explore only what the next ready ticket needs, optionally with an exploration
   subagent. Keep issue-linked notes and evidence in persistent task directories
   accessible to subsequent workers; follow `docs/agents/development.md` for
   bounded reads, check records and artifact custody.

3. Create the integration branch. If the issue tracker closes work through PRs, or the user asks for one, open a draft PR after the first merge in step 5 (a branch with no commits ahead of main can't open one), marked as closing the spec and tickets.

4. Implement ready tickets in dependency order. Delegate independent tickets when
   useful; each delegated implementer gets a fresh context and a persistent
   worktree/branch based on the integration tip. Preserve existing work when
   reconciling branches. Choose tests that prove the ticket's behavior, invoking
   `tdd` when appropriate, and incorporate the current integration tip before
   reporting completion.

5. The coordinator integrates completed work and resolves routine conflicts.
   Do not create a separate agent merely to execute merge commands. Follow the
   tracker's exact-candidate check and publication requirements.

6. Recompute the ready frontier and continue within the accepted scope and effort
   allowance. Parallelize only independent work with a concrete benefit.

7. Once the bounded candidate is complete, use `code-review` for fresh independent
   Standards and Spec review against the recorded fixed point. Fix actionable
   findings; the coordinator may make routine repairs. Review changed risk and
   unresolved findings without repeatedly restarting a whole-spec review.

8. If a draft PR exists, mark it ready for review. Otherwise, resolve each ticket the way the issue tracker closes work, and report the integration branch.

9. Publish and merge only within existing authorization, verify exact-head CI,
   and pull main as requested. Keep a concise outcome and remaining acceptance
   boundary in the issue. Remove task worktrees only after confirming all work is
   preserved and return-use evidence/launchers remain available.
