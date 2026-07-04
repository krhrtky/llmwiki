# Requirement 002: Human Agent Responsibility

## Background

LLMWiki は Agent が自律的に wiki を保守するが、人間の判断を置き換えるものではない。人間に残る役割は、問いの組み立て、判断、source 選定、公開承認である。

## Problem

Agent が実装や文書作成だけを高速化しても、判断材料が外化されなければレビュー不能になる。会話、設計意図、懸念、代替案が残らない場合、後続 Agent は同じ誤りを繰り返す。

## Goals

- 人間と Agent の責務境界を明示する。
- Agent の作業を、判断の代替ではなく判断材料の外化として位置づける。
- コードを書く前後に、前提、判断、懸念、影響範囲を残す。

## Non-goals

- Agent に最終ポリシー判断を委譲しない。
- Agent に秘匿情報公開の承認を委譲しない。

## User Value

人間は「この実装でよいか」だけでなく「そもそもこの問いでよいか」を判断できる。

## Acceptance Criteria

- 人間が担う責務と Agent が担う責務が分離されている。
- Agent が拒否・保留すべき条件が示されている。
- org publish には human review が必要と分かる。

## Related ADRs

- [ADR 001](../adr/001-use-docs-as-source-of-truth.md)
- [ADR 012](../adr/012-require-human-review-for-org-publish.md)

## Open Questions

- human reviewer の具体的なロール名と承認権限。
