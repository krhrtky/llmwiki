---
name: llmwiki-maintain
description: Use when maintaining an LLMWiki bundle by running llmwiki graph and lint, interpreting findings, and proposing reviewable fixes without directly mutating wiki content.
---

# LLMWiki Maintain

Inspect graph and lint health for an LLMWiki bundle.

## Workflow

1. Run graph generation for the relevant paths:

```bash
llmwiki graph --workspace-root . <paths>
```

2. Run lint:

```bash
llmwiki lint --workspace-root . <paths>
```

3. Interpret JSON `findings`, `edges`, `relations`, and orphan/broken link information.
4. For fixes, propose changes as reviewable edits or filing artifacts. Do not silently change lifecycle state.

## Stop Conditions

- Lint finding requires human decision.
- Relation storage format is ambiguous.
- A fix would alter source content or policy without an ADR or user decision.
