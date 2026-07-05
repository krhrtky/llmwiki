---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 019: Non-Goals

## Background

LLMWiki の初期実装は、SoT と運用モデルを固める段階である。最初から重い platform を作ると、形式より運用判断が曖昧になる。

## Non-goals

- vector DB を初期必須にしない。
- 全文検索 service を初期必須にしない。
- raw source の全文を org scope に集約しない。
- 秘匿情報を自動公開しない。
- train operation を初期許可しない。
- domain application の case、customer、SLA、assignment を LLMWiki Core に含めない。
- AGENTS.md に全仕様を詰め込まない。
- Agent に最終承認を委譲しない。

## Acceptance Criteria

- 実装者が初期範囲外の機能を判別できる。
- non-goal が ADR と矛盾していない。
- 後続実装で scope creep を検出できる。

## Related ADRs

- [ADR 008](../adr/008-separate-core-from-domain-apps.md)
- [ADR 011](../adr/011-start-with-file-and-cli-first.md)
