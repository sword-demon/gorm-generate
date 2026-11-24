# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the "Label in our tracker" column to match whatever vocabulary you actually use.

## Where a role is recorded

An issue file carries a `**Status:**` line near the top and a `**Category:**`
line. `Status:` is the single state field — it holds either a triage role or a
wayfinding lifecycle value, never both.

| Line | Vocabulary | Written by |
| ---- | ---------- | ---------- |
| `**Status:**` | one of the five roles above, or `claimed` / `resolved` | `/triage` applies a role; `/wayfinder` claims and resolves |
| `**Category:**` | `bug` / `enhancement` | `/triage` — what kind of work was requested |

`claimed` and `resolved` are wayfinding lifecycle values, not triage roles:
`claimed` means the ticket is being worked, `resolved` means it is done. They
sit outside the triage state machine, so a ticket in either state is not
triage work. A ticket with no `Status:` line has never been triaged — that is
the "unlabeled" bucket.

A ticket carries exactly one state value and one category role. When work
starts on a triaged ticket the state moves to `claimed`, and to `resolved` when
it lands; the triage role it held is then spent.

One field, not two, because the skills that create tickets fix the field name:
`/to-tickets` publishes `**Status:** ready-for-agent` and `/wayfinder` claims
with `Status: claimed`. Splitting the vocabulary across two lines would mean
every ticket published by `/to-tickets` landed the triage role in the wrong
field.
