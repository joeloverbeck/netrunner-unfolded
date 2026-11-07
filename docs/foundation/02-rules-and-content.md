# Rules and content

## Authority and reproducibility

Implement Null Signal's maintained Netrunner rules, with the explicit introductory teaching variant below. The [Comprehensive Rules hub](https://nullsignal.games/rules/comp-rules/) currently identifies **CR 26.03**, effective **2026-03-02**; [NetrunnerDB](https://netrunnerdb.com/) is its designated current card-text source. Current effective card text takes precedence where it modifies general rules. Official rulings clarify application; reconcile older rulings and obsolete step numbers with the chosen rules revision. Printed PDFs supply artwork, not authoritative current wording.

Implementation starts by refreshing and selecting a coherent rules/card-text/rulings snapshot. Record URLs, effective versions, source commits or hashes, and retrieval dates with the implementation. Do not silently upgrade active games when upstream changes. Pin the source basis in saves and fixtures. A source disagreement is investigated explicitly; convenient code behavior is not the oracle (`EVID`, `REP`, `OPS`).

Teaching references supplied by Jon remain required background:

- [Learn to play](https://nullsignal.games/players/learn-to-play/)
- [Corporation guide](https://nullsignal.games/players/learn-to-play/learn-to-play-corp/)
- [Runner guide](https://nullsignal.games/players/learn-to-play/learn-to-play-runner/)
- [Run timing guide](https://nullsignal.games/players/learn-to-play/run-guide/)

Use [the full rules](https://rules.nullsignal.games/) for exact priority, nested timing, and exceptions. Jinteki.net and Chiriboga can supply comparison cases, not normative answers. Rust effects are deliberately authored from reviewed rules; neither OCR nor a language model converts arbitrary card prose into authoritative executable behavior.

## Exact first content scope

Counts derive from the [official product deck lists](https://nullsignal.games/products/system-gateway/); identifiers were checked against the [pinned official card data](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/pack/sg.json). Codes below identify printings and are opaque strings, not an arithmetic identity scheme. Preserve logical card identity separately from printing/art and rules revision.

| Configuration | Runner deck | Corp deck | Agenda points to win |
| --- | ---: | ---: | ---: |
| Introductory | 30 | 34 | 6 |
| With advanced boosters | 40 | 44 | 7 |

Deck totals exclude identities. Runner identity: **The Catalyst: Convention Breaker**, `30076`. Corp identity: **The Syndicate: Profit over Principle**, `30077`. These teaching identities remain in both configurations; this is not a tournament-format deck validator. There are **43 distinct nonidentity cards plus two identities: 45 total**. The complete 77-card Gateway set is outside this milestone.

“Booster” is the number added to the introductory deck, not the advanced total.

| Runner card | Code | Intro | Booster |
| --- | --- | ---: | ---: |
| Tread Lightly | 30012 | 2 | 0 |
| Creative Commission | 30020 | 2 | 0 |
| VRcation | 30021 | 2 | 0 |
| Overclock | 30029 | 2 | 0 |
| Jailbreak | 30028 | 3 | 0 |
| Sure Gamble | 30030 | 3 | 0 |
| Docklands Pass | 30013 | 1 | 0 |
| Pennyshaver | 30014 | 1 | 0 |
| Cleaver | 30006 | 2 | 0 |
| Carmen | 30015 | 2 | 0 |
| Unity | 30026 | 2 | 0 |
| Mayfly | 30032 | 2 | 0 |
| Red Team | 30018 | 1 | 0 |
| Telework Contract | 30027 | 2 | 0 |
| Smartware Distributor | 30033 | 2 | 0 |
| Verbal Plasticity | 30034 | 1 | 0 |
| Wildcat Strike | 30002 | 0 | 2 |
| Mutual Favor | 30011 | 0 | 2 |
| DZMZ Optimizer | 30022 | 0 | 2 |
| Leech | 30008 | 0 | 2 |
| Conduit | 30024 | 0 | 2 |

| Corp card | Code | Intro | Booster |
| --- | --- | ---: | ---: |
| Offworld Office | 30067 | 3 | 0 |
| Send a Message | 30069 | 2 | 0 |
| Superconducting Hub | 30070 | 2 | 0 |
| Nico Campaign | 30037 | 2 | 0 |
| Urtica Cipher | 30045 | 2 | 0 |
| Regolith Mining License | 30071 | 2 | 0 |
| Brân 1.0 | 30039 | 2 | 0 |
| Diviner | 30046 | 2 | 0 |
| Karunā | 30047 | 2 | 0 |
| Palisade | 30072 | 3 | 0 |
| Tithe | 30073 | 2 | 0 |
| Whitespace | 30074 | 2 | 0 |
| Seamless Launch | 30040 | 2 | 0 |
| Government Subsidy | 30064 | 2 | 0 |
| Hedge Fund | 30075 | 3 | 0 |
| Manegarm Skunkworks | 30042 | 1 | 0 |
| Orbital Superiority | 30068 | 0 | 2 |
| Funhouse | 30054 | 0 | 2 |
| Public Trail | 30057 | 0 | 2 |
| Predictive Planogram | 30056 | 0 | 2 |
| Retribution | 30065 | 0 | 1 |
| AMAZE Amusements | 30058 | 0 | 1 |

## Coverage obligations

The supported rules include setup/mulligans; asymmetric turns; legal installation, rez, advancement, scoring and stealing; priority and trigger order; runs, subroutines, breaking and jack-out; all three central servers and remotes; breach/access ordering and information rights; economy, hosted counters, memory/uniqueness, tags, damage, searches/shuffles, once-per-turn and past-event conditions; and every terminal condition reachable here. Distinguish costs from effects and placing counters from advancing. These are coverage families, not a proposed universal card-language schema.

Create independently sourced expectations for each card and its interactions. Particularly valuable cases are Brân changing ICE during encounter; Funhouse's encounter trigger; Manegarm's approach timing; Overclock credits versus Whitespace; Conduit access counts; Mayfly's delayed trash; rezzed versus unrezzed AMAZE; opponent choices on Wildcat Strike; and immediate victory during Send a Message. [Evidence notes](../research/evidence.md#rules-interaction-evidence) link their rulings and describe traps. These examples do not exhaust the complete-deck obligation.

For each supported card, implementation evidence connects its source identity and revision to all effect clauses, choices, durations, and targeted cases. Keep this close to code/tests or the active issue; do not manufacture a parallel card-status bureaucracy. Cross-card tests derive from interactions, not only isolated happy paths. Reject unsupported cards/decks before a game begins. Adding art or metadata alone never admits a card to playable or bot-supported scope.
