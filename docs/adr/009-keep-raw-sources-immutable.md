# ADR 009: Keep Raw Sources Immutable

## Status

Accepted

## Context

LLMWiki は raw source を読み、wiki に要約・解釈・関係・矛盾を反映する。source 自体を変更すると、根拠と解釈の境界が曖昧になる。

## Decision

Raw source は immutable とし、LLM は変更しない。wiki は LLM-maintained な解釈層として更新する。

## Alternatives

- raw source を編集する: 誤字修正などは楽だが、根拠の真正性が失われる。
- source と wiki を同じ層に置く: 単純だが、証拠と解釈が混ざる。
- source は immutable、wiki は mutable: 根拠と解釈を分離できる。

## Rationale

source は証拠であり、wiki は compile された解釈である。この分離が citation、audit、contradiction detection の前提になる。

## Consequences

- Positive: claim の根拠を辿りやすい。
- Positive: source 更新ではなく source 追加として履歴を管理できる。
- Negative: source の正規化や redaction は別 layer で扱う必要がある。

## Related Requirements

- [Requirement 010](../requirements/010-source-evidence-and-citation.md)
