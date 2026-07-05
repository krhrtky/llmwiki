---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 018: Review and Ownership

## Background

LLMWiki は複数 scope と複数 owner を持つ。特に org scope の知識は横断影響を持つため、owner、reviewer、risk owner が必要である。

## Problem

owner がない page は更新責任が不明になり、reviewer がない policy は正当性を確認できない。

## Goals

- 必要な関係者と責務を定義する。
- org publish には human review を必須にする。
- risk owner が必要な知識種別を区別する。

## Roles

- `source_curator`: raw source を選定する。
- `wiki_maintainer`: wiki 更新を保守する。
- `page_owner`: page の正確性と更新責任を持つ。
- `domain_owner`: domain 横断の整合性を持つ。
- `team_owner`: team scope の公開責任を持つ。
- `reviewer`: 内容を確認する。
- `approver`: publish を承認する。
- `consumer`: wiki を利用する。
- `agent_operator`: Agent 実行を管理する。
- `risk_owner`: privacy、security、legal、人事などのリスクを判断する。

## Acceptance Criteria

- org publish の reviewer と approver が必要である。
- page owner と domain owner の違いが分かる。
- risk owner が redaction と access control に接続されている。

## Related ADRs

- [ADR 012](../adr/012-require-human-review-for-org-publish.md)
- [ADR 004](../adr/004-require-redaction-gate.md)
