---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 023: Use Storage Registry for Visibility Boundaries

## Status

Accepted

Supersedes [ADR 002](./002-adopt-personal-team-org-scope.md) and [ADR 019](./019-do-not-add-private-scope.md).

## Context

Operation-aware access control can decide whether an operation may read content, but it does not by itself prevent a command from traversing the wrong repository or bundle. LLMWiki needs a boundary where private knowledge, team knowledge, and org knowledge can be separated before scope evaluation.

The earlier `personal -> team -> org` scope model mixed the effective audience of knowledge with the physical location used to protect it. This became ambiguous once `team` needed multiple repositories and `org` was not yet mandatory.

## Decision

LLMWiki uses a root `llmwiki.yaml` storage registry to define visibility storage boundaries.

Storage visibility kinds are:

- `private`: at most one store. It may be local or repository-backed.
- `team`: multiple stores are allowed. Each store has a unique `team_id` and one repository.
- `org`: optional while org publishing remains under evaluation. If configured, at most one repository is allowed.

`private` is not a page-level scope. It is a storage visibility boundary. Existing `personal` page metadata is treated as migration input for the `private` store.

Repository identity and canonical root must be unique across configured stores. A `team` repository must not be reused by another `team` or by `org`.

## Alternatives

- Keep `personal -> team -> org` as logical scope and add storage as a second axis: preserves compatibility but keeps two overlapping classification systems.
- Use only scope rules: simple on paper, but physical traversal remains dependent on the caller passing the correct root.
- Require `org` immediately: over-constrains an area still under evaluation.

## Rationale

The storage registry makes the first security boundary explicit. CLI commands resolve `--store private`, `--store team:<team_id>`, or `--store org` to one canonical root before reading files. Operation-aware access control still applies inside the selected store.

Keeping `org` optional allows private and team use cases to work before org publishing policy is finalized.

## Consequences

- Positive: CLI operations cannot silently mix private, team, and org stores when using store selectors.
- Positive: multiple team repositories are represented without overloading `scope: team`.
- Positive: org can be introduced later without changing private/team store semantics.
- Negative: existing `workspace_root` / `scope` command inputs need a migration period.
- Negative: scope rules, scope evaluations, proposal drafts, graph/export artifacts need store identity fields.

## Related Requirements

- [Requirement 004](../requirements/004-scope-model.md)
- [Requirement 006](../requirements/006-propose-workflow.md)
- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
