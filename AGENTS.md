## Agent skills

### Issue tracker

Issues live as markdown files under `.scratch/<feature>/` (local tracker). See `docs/agents/issue-tracker.md`.

### Triage labels

Default five role strings, recorded in each issue's `Status:` line — the single state field, which also carries the wayfinding lifecycle values `claimed`/`resolved` — alongside a `Category:` line for `bug`/`enhancement`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

Documented solutions live under `docs/solutions/`, organized by category with YAML frontmatter (`module`, `tags`, `problem_type`); `CONCEPTS.md` contains shared project vocabulary. These are relevant when implementing, debugging, or making design decisions in documented areas.
