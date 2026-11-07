# Triage labels

Use these mappings whenever an engineering skill names a triage role.

| Canonical role | Tracker label | Meaning |
| --- | --- | --- |
| `needs-triage` | `needs-triage` | Maintainer needs to evaluate the issue |
| `needs-info` | `needs-info` | Waiting for information needed to triage |
| `ready-for-agent` | `ready-for-agent` | Specified and feasible for bounded agent execution |
| `ready-for-human` | `ready-for-human` | Requires human implementation |
| `wontfix` | `wontfix` | Will not be actioned |

Before applying `ready-for-agent`, satisfy the repository's `WORK-1` and `OPS-1` requirements. The label records readiness; it does not authorize execution, unattended continuation, publication, or acceptance. Follow the human-approved scope and the applicable constitution gates even when a skill recommends applying readiness automatically.

Edit the tracker-label column if the repository adopts different names.
