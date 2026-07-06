---
name: llmwiki-file-knowledge
description: Use when turning LLMWiki query or ingest output into a reviewable filing artifact by deriving and running llmwiki file with owner, reviewer, citations, and access policy references.
---

# LLMWiki File Knowledge

Prepare a candidate for review rather than writing directly into the wiki.

## Workflow

1. Start from an existing candidate path from `query`, `ingest`, or a user-provided draft.
2. Confirm `scope`, `owner`, `confidence`, at least one `citation`, and at least one `access_policy_ref`.
3. For `team` or `org`, also require `reviewer`.
4. Run:

```bash
llmwiki file --workspace-root . --candidate <candidate.md> --scope <personal|team|org> --owner <owner> --reviewer <reviewer> --confidence <high|medium|low> --citation "<citation>" --access-policy-ref <policy-id>
```

5. Report the filing artifact and any missing metadata from the JSON result.

## Stop Conditions

- Owner, required reviewer, citation, confidence, or access policy reference is missing.
- Sensitive category exists but no risk owner is available.
- The user asks to bypass review and write directly to canonical wiki pages.
