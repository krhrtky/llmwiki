---
name: llmwiki-export
description: Use when exporting an LLMWiki bundle or scope by deriving and running llmwiki export with scope, content level, subject, and access policy checks.
---

# LLMWiki Export

Export a bundle or selected scope with access control.

## Workflow

1. Confirm scope or explicit paths.
2. Confirm `content_level`, `subject_kind`, `subject_id`, and access policy file.
3. Run:

```bash
llmwiki export --workspace-root . --scope <personal|team|org> --content-level <metadata|summary|content> --subject-kind <kind> --subject-id <id> --access-policy <policy.yaml>
```

For explicit paths, pass paths instead of or in addition to scope according to the current CLI contract.
4. Report exported artifact path, manifest, and denied/held content from the JSON result.

## Stop Conditions

- Access policy is unavailable.
- Export includes content outside the allowed scope or content level.
- The requested destination is outside the workspace boundary.
