# ADR 003: Use Propose Not Backport

## Status

Accepted

## Context

下位スコープの知識を上位スコープへ移す際、copy や backport という言葉は、文脈や秘匿情報をそのまま上位へ移す印象を与える。

## Decision

上位スコープへの昇格候補提出を `propose` と呼ぶ。

## Alternatives

- `backport`: code branch の逆流のように見え、知識の抽象化を表現しにくい。
- `promote`: 昇格完了まで含む印象があり、review 前の提出を表現しにくい。
- `propose`: 提案であり、review、reject、approve を自然に表現できる。

## Rationale

`propose` は、人間が判断する前の候補状態を明確にする。下位知識を compile し、抽象化・匿名化・根拠整理したうえで提出する意味を持たせられる。

## Consequences

- Positive: publish と proposal を分離できる。
- Positive: reject や hold が自然に扱える。
- Negative: propose object と workflow state が必要になる。

## Related Requirements

- [Requirement 006](../requirements/006-propose-workflow.md)
