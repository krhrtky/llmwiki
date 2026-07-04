# ADR 004: Require Redaction Gate

## Status

Accepted

## Context

personal や team の知識には、人事情報、個人情報、顧客固有情報、契約情報、認証情報、未公開事業情報が含まれる可能性がある。

## Decision

`propose` には redaction / generalization gate を必須にする。

## Alternatives

- reviewer の目視だけにする: 低コストだが漏えい検出が属人的になる。
- publish 後に検出する: 被害範囲が拡大する。
- propose 前後に gate を置く: 実装コストはあるが、上位 scope への混入を防ぎやすい。

## Rationale

LLMWiki は graph、search、export、train と接続される可能性がある。秘匿情報は一度混入すると派生物に広がるため、上位 scope へ入る前に止める必要がある。

## Consequences

- Positive: 秘匿情報の上位 scope 混入を抑制できる。
- Positive: redaction report により reviewer が判断しやすい。
- Negative: false positive により propose が遅くなる可能性がある。

## Related Requirements

- [Requirement 007](../requirements/007-redaction-and-generalization.md)
- [Requirement 018](../requirements/018-review-and-ownership.md)
