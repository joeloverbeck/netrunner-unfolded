# Basic Corp opponent

[Issue #24](https://github.com/joeloverbeck/netrunner-unfolded/issues/24) introduced
`CorpV1`. [Issue #26](https://github.com/joeloverbeck/netrunner-unfolded/issues/26)
repairs Archives candidate lookup as `CorpV2`, selected for new introductory games
where the human plays Runner. Brân evaluates affordable inward ICE from both HQ
and Archives against the same later rez reserve, including Tread Lightly's tax. The ordinary
[setup and saved-return path](introductory-match.md) is unchanged. The opponent
uses only its current Corp observation, including public board installation
timing. It does not receive a match, save, gameplay seed, future draw order,
Runner secrets, or private diagnostic record.

The policy is deterministic and stateless. Its plans are reconstructed from the
current lawful board; it has no unused memory or policy random stream to persist.
The save records its policy version and source identity alongside the game and
human seat in the same atomic replacement. Saves from before the shared save
record (unfolded-tabletop #36), including introductory format 1 saves, are refused
as unsupported and left unchanged; none is migrated. A changed or unsupported
policy identity refuses loading. `CorpV0` and `RunnerV0` remain
callable with their original source hashes. `CorpV1` is also retained byte-for-byte
in `src/policies/corp_v1.rs`, and existing saves keep their recorded policy.
These versions are comparison incumbents, not aliases for the current selector.

The priorities are game-specific heuristics over the admitted introductory cards,
not a general rules-text interpreter or strength evaluator:

| Decision family | Deliberate handling |
| --- | --- |
| Opening hand | Keep affordable defense or economy; otherwise use the once-only mulligan. |
| Scoring and advancement | Score the largest available agenda; finish an affordable agenda this turn; use an eligible Seamless Launch for two needed counters; otherwise build advancement. |
| Defense | Protect HQ agendas, R&D and scoring remotes; consider existing ICE layers, installation cost and rez reserves; use a protected empty remote for the next agenda. |
| Economy | Play affordable Hedge Fund/Government Subsidy, take Regolith credits, install campaigns when rez reserves allow, and draw toward a scoring plan when the economy can support it. |
| Rez and access reactions | Rez legally approached ICE; reserve credits before activating economy; activate Manegarm on its attacked server; leave Urtica unrezzed because its access damage already works. Damage itself is engine bookkeeping. |
| Brân | Install affordable inward ICE for free, retain existing defense when completion permits, or deliberately decline; installation never supplies a free rez. |
| Agenda rewards | Use Send a Message's free rez on costly available ICE; decline Hub draw when R&D cannot safely supply it or the hand is already stocked. |
| Installation and targets | Prefer vacant protected roots, put Manegarm with an agenda, remove a required displaced card, and complete installation without gratuitous trashing. Seamless targets prefer needed agenda counters. |
| Income and discard | Resolve mandatory income; select lower retained-value hand cards and confirm the required discard. |
| Remaining ordinary options | Pass paid windows when spending buys no selected benefit. Extra traps, upgrades, ICE and useless virus purges lose to productive actions or saving credits. |

These priorities do not know whether unknown Runner cards will defeat a server,
whether a trap will be accessed, or the order of either deck. They do not claim
optimal bluffing, exact tactical search or experienced-casual play.

Native decisions run in a worker that receives only an observation. Every result
is bound to its originating controller, actor, revision, policy and cancellation
epoch. Correction, refusal, a new game or exit invalidates old work. The render
thread polls without waiting; a nineteen-second budget leaves headroom within
the twenty-second ceiling. The finite selector normally needs much less time.
Cancellation and deadline checks surround the bounded selector, which has no
search or external calls. A timeout or unavailable result uses a labelled legal
emergency fallback; normal strategy has its own explicit choice handling.

The existing agent seat exposes the policy identity, elapsed time, invocation
count when known, outcome and commit status. It excludes the opponent's action
and private legal-choice count. Diagnostics record lawful inputs separately from
private reproduction data. Unknown discarded worker work is reported as unknown.
Failed persistence leaves the committed game and policy intact and stops further
play until reopening. No new thinking-screen design is introduced.

`tests/corp_policy.rs` supplies source-derived scoring, credit, timing, defense,
replacement and pending-choice examples. `tests/opponent.rs` exercises actual
proposal/commit, worker cancellation, timeout fallback, save failure and migration.
Both-role hidden-state permutations in `tests/diagnostics.rs` include Corp-v1.
These tests establish their stated cases, not general playing strength. The
comparison runner's independent quality checks remain limited to imminent agenda
victory, unsafe Hub draw and exposed Karunā lethal damage; other decisions remain
unassessed in its report. Keep completion, legality/privacy, mistakes, unassessed
choices, latency and work separate.

The comparison CLI accepts `CorpV1` in the existing `compare PLAN FRESH-DIRECTORY`
declaration. Change one role at a time against frozen Runner-v0; preserve source,
executable and plan hashes and reproduce retained records with that exact binary.
Held-out seeds are used only after freezing the candidate and cannot be reused
as held-out evidence after tuning. The bounded allowance and actual findings
belong to #24, not this guide.

Source refresh on 2026-11-15: the [official rules hub](https://nullsignal.games/rules/comp-rules/)
still identifies CR 26.03, and the official card-data HEAD remains the accepted
`6ff880724741ea6ac062acf77e0cf979bb44e67e`. The [Corp guide](https://nullsignal.games/players/learn-to-play/learn-to-play-corp/)
supplies the economic/defensive/scoring task. The pinned
[Seamless Launch](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/seamless_launch.json)
and [Nico Campaign](https://github.com/Null-Signal-Games/netrunner-cards-json/blob/6ff880724741ea6ac062acf77e0cf979bb44e67e/v2/cards/nico_campaign.json)
clauses were checked directly. The existing source precedence and admitted
content topology remain unchanged.
