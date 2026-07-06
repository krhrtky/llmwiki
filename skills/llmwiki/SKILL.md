---
name: llmwiki
description: LLMWiki skill suite の入口。用途別 skill の選択、SoT 確認、CLI-first の実行境界を案内する。
---

# LLMWiki

LLMWiki の作業を始める入口です。まず repository root の `AGENTS.md` を確認し、必要に応じて `docs/index.md`、`docs/glossary.md`、`docs/requirements/index.md`、`docs/adr/index.md`、`docs/open-questions.md` を読む。

## 停止条件

- `docs/` の要求や ADR と矛盾する場合
- 未決事項を推測で埋める必要がある場合
- 変更範囲が `docs/` の SoT を越える場合

## 用途別 skill

- 質問へ回答する: `llmwiki-answer-query`
- raw source から candidate を作る: `llmwiki-ingest-source`
- candidate を review 用 filing にする: `llmwiki-file-knowledge`
- 下位 scope から上位 scope へ昇格提案する: `llmwiki-promote-knowledge`
- graph/lint で bundle を保守する: `llmwiki-maintain`
- scope または bundle を export する: `llmwiki-export`

LLMWiki Core の操作は `ingest`、`query`、`related`、`file`、`lint`、`graph`、`propose`、`redact`、`export` です。用途別 skill はユーザー意図から必要な Core operation と CLI 引数を導出して実行します。

## Distribution helper

```bash
llmwiki skill install --workspace-root . [--codex-home <path>]
```

`skill install` は `skills/*/SKILL.md` を Codex skill directory へ配置する配布補助で、Core knowledge operations には含めません。

## 使い方

1. 実装や運用の前に `docs/` の該当文書を読む。
2. 判断が分岐する場合は、ユーザー確認か ADR 更新を優先する。
3. LLMWiki 固有の変更は file-first と CLI-first を守る。
