# ADR 005: Use Operation-Aware Access Control

## Status

Accepted

## Context

公開範囲だけでは、metadata は見せるが content は見せない、graph edge は使うが retrieve は不可、query には使うが train は不可、といった制御を表現できない。

## Decision

LLMWiki は operation-aware access control を採用する。初期 docs では概念モデルまでを固定し、policy schema と評価順序は後続 ADR で決める。

## Alternatives

- `visibility` のみ: 単純だが操作差を表現できない。
- RBAC のみ: role と operation の関係は表せるが、content level や scope 差を表しにくい。
- operation-aware model: 実装は複雑だが、LLMWiki の利用形態に合う。

## Rationale

LLMWiki は human reader、Agent、CLI、graph builder、exporter、training pipeline など異なる consumer を持つ。consumer ごとではなく operation ごとに制御する方が、意図しない流用を防げる。

## Consequences

- Positive: 低リスク操作と高リスク操作を分離できる。
- Positive: `train` や `export` など高影響操作を明示的に制御できる。
- Negative: 認可判断と audit log の設計が必要になる。

## Related Requirements

- [Requirement 008](../requirements/008-operation-aware-access-control.md)
