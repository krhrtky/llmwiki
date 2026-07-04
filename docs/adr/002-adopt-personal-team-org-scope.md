# ADR 002: Adopt Personal Team Org Scope

## Status

Accepted

## Context

知識は個人、チーム、組織横断で有効範囲が異なる。中央集権だけでは秘匿性と局所性を扱いにくく、完全分散だけでは共通語彙と制約が共有されない。

## Decision

知識スコープとして `personal → team → org` を採用する。

## Alternatives

- `central` を使う: 中央集権の保存場所に見えやすい。
- `global` を使う: 全世界または全システムに有効という誤解がある。
- `org` を使う: 組織横断の正規知識を表現しやすい。

## Rationale

`org` は全情報の中央保存ではなく、組織横断の正規知識スコープとして扱える。personal と team の局所性を保ったまま、横断制約を表現できる。

## Consequences

- Positive: 知識の成熟度と公開範囲を分けられる。
- Positive: team 固有知識を無理に org へ集約しなくてよい。
- Negative: scope 間の proposal、link、review の実装が必要になる。

## Related Requirements

- [Requirement 004](../requirements/004-scope-model.md)

## References

- [LLM Wiki](../references/index.md#llm-wiki)
