---
name: llmwiki
description: LLMWiki の操作入口と最小手順を案内する skill
---

# LLMWiki

LLMWiki の作業を始める入口です。まず repository root の `AGENTS.md` を確認し、必要に応じて `docs/index.md`、`docs/glossary.md`、`docs/requirements/index.md`、`docs/adr/index.md`、`docs/open-questions.md` を読む。

## 停止条件

- `docs/` の要求や ADR と矛盾する場合
- 未決事項を推測で埋める必要がある場合
- 変更範囲が `docs/` の SoT を越える場合

## Core knowledge operations

LLMWiki Core の操作は `ingest`、`query`、`related`、`file`、`lint`、`graph`、`propose`、`redact`、`export` です。

## Distribution helper

```bash
llmwiki skill install --workspace-root . [--codex-home <path>]
```

`skill install` は `skills/llmwiki/SKILL.md` を Codex skill directory へ配置する配布補助で、Core knowledge operations には含めません。

## 使い方

1. 実装や運用の前に `docs/` の該当文書を読む。
2. 判断が分岐する場合は、ユーザー確認か ADR 更新を優先する。
3. LLMWiki 固有の変更は file-first と CLI-first を守る。
