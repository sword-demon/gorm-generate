# Issue tracker: GitHub Issues for reports, local Markdown for design work

The tracker is split by audience:

- **Public reports** — bug reports and feature requests from users — live in GitHub Issues. New issues go through the forms in `.github/ISSUE_TEMPLATE/` (bug: version, dialect, cache flag, redacted minimal DDL; feature: problem, proposal, alternatives). The app's header button opens the bug form prefilled with version, dialect and the cache flag. Triage and dedup happen on GitHub.
- **Design-time tickets and specs** — anything the skills below write while exploring the codebase — stay as markdown files in `.scratch/`. The rest of this document describes that local tracker.

## Conventions

Design issues and specs for this repo live as markdown files in `.scratch/`.

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`, never a single combined tickets file
- Each issue file records a `**Status:**` line near the top — the single state field, holding either a triage state role or the wayfinding lifecycle values `claimed`/`resolved` — plus a `**Category:**` line (`bug`/`enhancement`) for the kind of work; see `triage-labels.md` for the role strings
- Comments and conversation history append to the bottom of the file under a `## Comments` heading

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` (the Notes / Decisions-so-far / Fog body).
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `Status:` line records `claimed`/`resolved`.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.

## Rejected requests

Requests that were considered and turned down are recorded in
`.out-of-scope/<concept>.md` — one file per rejected concept, so a later
triage pass recognizes a repeat instead of re-litigating it. The directory
currently holds no records (the placeholder file is empty); the file format is
described in the triage skill's `OUT-OF-SCOPE.md`.
