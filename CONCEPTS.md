# Concepts

Shared domain vocabulary for this project — entities, named processes, and status concepts with project-specific meaning. Seeded with core domain vocabulary, then accretes as ce-compound captures and refreshes learnings; direct edits are fine. Glossary only, not a spec or catch-all.

## Generation core

### Resolved Table
The complete set of Go-facing facts derived from one parsed DDL table before rendering, including names, types, imports, primary-key information, cache-key format, comments, and warnings.

### Layer Contract
The shared method signatures and primary-key argument expressions used by the repository, service, and cache decorator for one resolved table.

### Table Output
The complete generated contribution of one table: its files, table-scoped warnings, and the third-party dependencies those files require.

### Renderer
A component that turns resolved table facts into one part of the generated Go project while preserving ownership of that part's output format.

## Relationships

A Resolved Table feeds a Renderer. Renderers share a Layer Contract but retain ownership of their own generated output. A renderer set is assembled into one Table Output.
