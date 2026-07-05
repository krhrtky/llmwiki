---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 021: Trace Docs and Implementation with Stable Evidence Links

## Status

Accepted

## Context

LLMWiki の SoT は docs にあるが、実装の判断根拠がコード、テスト、CI、fixture、コマンド例に散らばると、どの docs がどの実装を支えるかを追いにくい。handoff や一時ログに依存すると、証跡が期限切れになりやすい。

## Decision

初期 traceability は、implementation artifact と直接対応する requirement / spec を対象にする。対応関係の正本は page 隣接の `*.llmwiki.yaml` にある `relations[]` とし、`implemented_by`、`verified_by`、`enforced_by`、`distributed_as` で実装、検証、強制、配布を追跡する。

Markdown link は docs 間の navigation と根拠参照の正本であり、sidecar relation は docs と implementation artifact の機械可読な対応の正本である。ADR と milestones は、対応する requirement / spec または sidecar relation を通じて追跡し、初期段階では全 ADR と milestones に個別 sidecar relation を必須化しない。

作業状況メモや一時ログは SoT に含めない。次 ToDo や進行中メモは成果物に混ぜない。

## Alternatives

- handoff 文書で追う: その場では便利だが、更新漏れと期限切れが起きやすい。
- 追跡しない: docs と実装のズレを発見しにくい。
- Markdown link だけで実装 artifact まで追う: docs 本文が実装一覧で肥大化しやすく、コードや test への target 種別も表現しにくい。

## Rationale

sidecar relation に実装証跡を置くと、本文を作業管理表にせずに、Agent が要件、仕様、実装、検証の対応関係を機械的に辿れる。これにより、docs を読む入口と、実装検証の入口を分けつつ、両者の整合を保ちやすくなる。

## Consequences

- Positive: docs と implementation の対応関係をたどれる。
- Positive: handoff や一時ログへの依存を減らせる。
- Positive: milestone の証跡をレビューしやすくなる。
- Negative: docs 更新と実装更新の両方に cross-link の管理コストが発生する。

## Related Requirements

- [Requirement 011](../requirements/011-graph-and-relation.md)
- [Requirement 017](../requirements/017-harness-engineering.md)
- [Requirement 020](../requirements/020-implementation-milestones.md)
