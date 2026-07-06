---
name: llmwiki-ingest-source
description: Use when converting raw source files into LLMWiki candidate pages by deriving and running llmwiki ingest while preserving immutable raw sources and citations.
---

# LLMWiki Ingest Source

Create wiki candidates from raw source without modifying the source.

## Workflow

1. Read repository `AGENTS.md` and relevant requirement or ADR before changing repository content.
2. Confirm the source path is inside the workspace and the target scope is `personal`, `team`, or `org`.
3. Run:

```bash
llmwiki ingest --workspace-root . --scope <personal|team|org> <source-path>
```

4. Inspect the JSON result: `candidates`, `evidence_map`, `artifact_path`, and `diff_summary`.
5. If the user wants the candidate prepared for review, continue with `llmwiki-file-knowledge`.

## Stop Conditions

- The operation would overwrite raw source.
- Scope is unknown.
- Source path is outside the workspace or cannot be read.
- Evidence or citation cannot be produced.
