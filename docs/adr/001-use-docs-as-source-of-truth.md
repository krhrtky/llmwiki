# ADR 001: Use Docs as Source of Truth

## Status

Accepted

## Context

LLMWiki 実装は、会話ログ、外部参照、ADR、requirements、未決論点を継続的に扱う。AGENTS.md に全仕様を詰め込むと、context を圧迫し、腐敗し、機械的検証が難しくなる。

## Decision

LLMWiki 実装の Source of Truth は `docs/` に置く。AGENTS.md は入口と運用原則に限定し、詳細仕様は `docs/` へ誘導する。

## Alternatives

- AGENTS.md に全仕様を書く: Agent が毎回読めるが、長大化して重要度が薄まり、更新検証が困難になる。
- 外部ドキュメントに置く: 人間には読みやすいが、Agent 実行時に repository-local な文脈として扱いにくい。
- docs を SoT にする: git diff、review、link、lint が使える。

## Rationale

Harness Engineering の観点では、Agent が読めない文脈は実行時に存在しない。repository-local な Markdown は human-readable、agent-readable、diffable である。

## Consequences

- Positive: 実装者と Agent が同じ SoT を参照できる。
- Positive: ADR、requirements、open questions を分割して保守できる。
- Negative: docs の腐敗を防ぐ lint と gardening が必要になる。

## Related Requirements

- [Requirement 001](../requirements/001-vision-and-problem.md)
- [Requirement 017](../requirements/017-harness-engineering.md)

## References

- [OpenAI Harness Engineering](../references/index.md#openai-harness-engineering)
