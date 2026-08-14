---
name: llmwiki
description: Maintain persistent, source-backed LLM Wikis with the llmwiki CLI. Use when a user wants to initialize a knowledge wiki in a repository, register and ingest local sources, query or lint that wiki, or validate its structure.
---

# LLM Wiki

Use `llmwiki` as the structural authority and the target Wiki's `AGENTS.md` as
the local knowledge-model authority. Do not change `raw/` or
`.llmwiki/manifest.json` except through the CLI.

## Select the Wiki

1. Use the target the user specifies. Otherwise, look for an initialized
   `knowledge/` directory in the current repository.
2. If none exists, propose or create `knowledge/` with `llmwiki init knowledge`.
   Do not run `init` at a non-empty repository root.
3. Confirm that the CLI is available and that `llmwiki --help` shows `init`,
   `source`, `search`, and `check`. If another command owns this name, do not
   overwrite it or assume it is compatible; ask the user for the path to this
   CLI or a preferred command name. If it is absent, install the released
   binary or build the intended source with `cargo install --path .`.
4. Read `TARGET/AGENTS.md` before changing the Wiki. Its local taxonomy,
   source policy, and privacy rules override generic conventions here.

## Ingest a Source

1. Register a source outside `TARGET/raw/` with
   `llmwiki source add TARGET FILE...`. Never copy or edit raw files directly.
2. Read the registered raw source and `TARGET/wiki/index.md`.
3. Follow the local `AGENTS.md` to create or update its source page, integrate
   supported claims into relevant knowledge pages, preserve contradictions, and
   update the index and append-only log.
4. Run `llmwiki check TARGET`. Resolve every structural error before reporting
   completion.
5. Report the registered source and the pages added or changed. Distinguish
   sourced findings from uncertainty.

## Query or Lint

- For a query, read `wiki/index.md` first, follow page links to source pages
  and raw sources, and state the supporting evidence and uncertainty. Save only
  reusable analysis when the local policy permits it.
- For a semantic lint, inspect contradictions, stale claims, unlinked concepts,
  and weak provenance. Make only changes authorized by the local `AGENTS.md`.
- After editing a Wiki, append the required log entry and run
  `llmwiki check TARGET`.

## Boundaries

- Treat raw sources as immutable and preserve the log as append-only.
- Do not invent source content, citations, or resolution of a contradiction.
- Keep repository-specific instructions in `TARGET/AGENTS.md`; keep reusable
  CLI workflow in this Skill.
