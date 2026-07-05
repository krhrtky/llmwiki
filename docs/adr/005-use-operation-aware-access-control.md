# ADR 005: Use Operation-Aware Access Control

## Status

Accepted

## Context

公開範囲だけでは、metadata は見せるが content は見せない、graph edge は使うが retrieve は不可、query には使うが train は不可、といった制御を表現できない。

## Decision

LLMWiki は operation-aware access control を採用する。初期 docs では concept model と policy object / decision log の最小契約項目までを固定する。認可エンジンの完全な policy schema は後続設計に残し、評価順序と競合解決は [ADR 016](./016-finalize-access-policy-evaluation.md) で固定する。

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

## Related ADRs

- [ADR 016](./016-finalize-access-policy-evaluation.md)
