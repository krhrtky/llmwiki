---
name: llmwiki-promote-knowledge
description: Use when promoting LLMWiki knowledge from personal to team or team to org by running redaction before propose and enforcing reviewer, approver, and human review requirements.
---

# LLMWiki Promote Knowledge

Promote lower-scope knowledge through redaction and proposal.

## Workflow

1. Confirm `from_scope` and `to_scope` form an upward move: `personal -> team`, `team -> org`, or `personal -> org`.
2. Run redaction/generalization first:

```bash
llmwiki redact --workspace-root . --target-scope <team|org> <page.md>
```

3. Inspect the redaction result. Continue only if residual risk is acceptable and a report path is available.
4. Require `reviewer` and `approver`; for `org`, human review is mandatory.
5. Run:

```bash
llmwiki propose --workspace-root . --from-scope <personal|team> --to-scope <team|org> --reviewer <reviewer> --approver <approver> --redaction-report <report.json> <page.md>
```

6. Report proposal draft, evidence map, and review requirements.

## Stop Conditions

- The move is not upward.
- Redaction/generalization is missing or leaves unresolved risk.
- Reviewer or approver is missing.
- `org` publication is requested without human review.
