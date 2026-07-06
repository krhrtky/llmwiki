---
name: llmwiki-answer-query
description: Use when answering a user question from an LLMWiki bundle by deriving and running llmwiki query or related with scope, content level, subject, and access policy checks.
---

# LLMWiki Answer Query

Answer from an existing LLMWiki bundle through the CLI.

## Workflow

1. Read repository `AGENTS.md`, then the relevant docs entrypoint when needed.
2. Choose `query` for keyword/content lookup; choose `related` when the user gives a seed page or asks for adjacent knowledge.
3. Require or discover access policy input before reading protected content. If policy is missing and the command requires it, stop and ask for the policy file or subject/scope.
4. Run the CLI, not ad-hoc file search, for the knowledge operation:

```bash
llmwiki query --workspace-root . --question "<question>" --scope <personal|team|org> --content-level <metadata|summary|content> --subject-kind <kind> --subject-id <id> --access-policy <policy.yaml>
```

For relation traversal:

```bash
llmwiki related --workspace-root . --seed <page.md> --scope <personal|team|org> --operation <operation> --content-level <metadata|summary|content> --subject-kind <kind> --subject-id <id> --access-policy <policy.yaml>
```

5. Base the answer on the JSON result: `answer`, `citations`, `confidence`, `matched_pages`, `results`, and `decision_logs`.

## Stop Conditions

- Access policy, subject, scope, or content level is required but unavailable.
- CLI returns `deny`, `hold`, or no sufficient citations.
- The user asks the query operation to write or publish content.
