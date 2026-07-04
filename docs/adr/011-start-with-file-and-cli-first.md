# ADR 011: Start with File and CLI First

## Status

Accepted

## Context

LLMWiki は将来的に API、DB、search、graph、workflow engine を持ち得る。しかし初期段階では SoT と運用モデルを固めることが重要である。

## Decision

初期実装は file-first、CLI-first で開始する。DB や vector DB は初期必須にしない。

## Alternatives

- 最初から DB 中心: workflow 管理はしやすいが、Markdown-first の可読性が下がる。
- 最初から vector DB 中心: retrieval は強いが、wiki maintenance の価値が薄れる。
- file + CLI first: git、diff、review、Agent 実行との相性がよい。

## Rationale

LLMWiki の初期価値は、知識を readable かつ diffable な形で蓄積することである。検索基盤は成長後に追加できる。

## Consequences

- Positive: すぐに Markdown と git で運用できる。
- Positive: Agent が shell から操作しやすい。
- Negative: 大規模化した場合は search/index/store の追加が必要になる。

## Related Requirements

- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 019](../requirements/019-non-goals.md)
