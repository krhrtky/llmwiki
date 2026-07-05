---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 010: Use Index and Log for Progressive Disclosure

## Status

Accepted

## Context

Agent が wiki 全体を一度に読むと context を浪費する。LLMWiki と OKF は `index.md` を入口にし、必要な bundle では `log.md` で履歴理解を補助する。

## Decision

各 bundle または重要 directory で `index.md` を使う。`log.md` は履歴運用が必要な場合に使う予約ファイルとする。

## Alternatives

- 全文検索だけに頼る: 小規模では過剰で、index による全体把握が弱い。
- README だけにまとめる: 履歴や directory ごとの progressive disclosure が弱い。
- `index.md` と `log.md` を使う: 人間と Agent の両方が読みやすい。

## Rationale

`index.md` は内容指向の navigation、`log.md` は時系列の変更履歴として機能する。OKF でも予約ファイルとして扱われるが、LLMWiki では log を必須にせず、必要な bundle だけに履歴管理を持たせる。

## Consequences

- Positive: Agent が最初に読む入口を小さくできる。
- Positive: 変更履歴と最近の作業を辿れる。
- Negative: index の更新漏れを lint する必要がある。
- Negative: log を運用する bundle では更新漏れを lint する必要がある。

## Related Requirements

- [Requirement 011](../requirements/011-graph-and-relation.md)
- [Requirement 016](../requirements/016-lint-and-gardening.md)
