---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 012: Agent Skills

## Background

LLMWiki は Agent が継続的に保守する。Agent が実行すべき操作を skill として明確にしないと、毎回 ad-hoc な作業になる。

## Problem

操作の目的、入力、出力、拒否条件がない場合、Agent は source copy、秘匿情報混入、link 更新漏れ、根拠漏れを起こす。

## Goals

- Agent Skill の初期一覧を定義する。
- 各 skill は入力、出力、拒否条件、更新対象を持つ。
- 実装フェーズでは skill ごとに CLI/API と接続する。
- repository から Codex Skill として配布できるようにする。

## Initial Skills

- `llmwiki`
- `llmwiki-answer-query`
- `llmwiki-ingest-source`
- `llmwiki-file-knowledge`
- `llmwiki-promote-knowledge`
- `llmwiki-maintain`
- `llmwiki-export`

### Core Operation Skills

- `ingest_source`
- `triage_knowledge`
- `answer_query`
- `follow_related`
- `nominate_for_promotion`
- `propose_knowledge`
- `generalize_for_upper_scope`
- `redact_sensitive_information`
- `map_evidence`
- `detect_conflicts`
- `route_reviewers`
- `build_graph`
- `lint_graph`
- `promote_knowledge`
- `deprecate_or_link_source_page`

## Acceptance Criteria

- skill が propose、redaction、graph、lint に対応している。
- skill は人間判断が必要な場面で停止できる。
- 実装モデル制約 `model: gpt-5.4 medium` が実装フェーズに適用される。
- `skills/*/SKILL.md` が repository 内にあり、`llmwiki skill install` で Codex skill directory へ配置できる。

## Related ADRs

- [ADR 001](../adr/001-use-docs-as-source-of-truth.md)
- [ADR 011](../adr/011-start-with-file-and-cli-first.md)
- [ADR 022](../adr/022-distribute-codex-skills-by-responsibility.md)
