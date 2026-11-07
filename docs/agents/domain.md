# Domain docs

## Layout and reading

This is a single-context repository:

- `GLOSSARY.md` at the repository root defines domain vocabulary.
- `docs/adr/` holds architecture decision records.

Before exploring a domain area, read the glossary if present and the ADRs relevant to that area. Existing `docs/foundation/` documents remain the source of accepted product scope and constraints.

If the glossary or ADR directory is absent, proceed silently. Create them lazily when authorized domain work resolves a term or requires an ADR; do not create placeholders during setup.

## Vocabulary and decisions

Use the glossary's terms in issue titles, proposals, hypotheses, and tests. If a needed concept is missing, reconsider whether the new term is necessary and record a genuine gap during authorized domain-modeling work.

Surface conflicts with an existing ADR explicitly, identifying the decision and why it may need to be reopened. A proposal does not silently override an accepted decision. Glossaries and ADRs record terminology and decisions; active work remains in GitHub Issues and PRs.
