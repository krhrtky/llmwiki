---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 007: Extend OKF with LLMWiki Namespace

## Status

Accepted

## Context

LLMWiki には scope、owner、access policy、sensitivity、proposal など OKF 本体にない情報が必要である。一方で OKF の最小性は壊したくない。

## Decision

LLMWiki 固有メタデータは frontmatter の `llmwiki` 配下に置く。

## Alternatives

- top-level key を増やす: 短く書けるが OKF の一般 key と衝突しやすい。
- 別 metadata file に分ける: OKF concept と metadata の同期が必要になる。
- `llmwiki` namespace に閉じ込める: 拡張範囲が明確になる。

## Rationale

OKF は producer-defined key を許容する。`llmwiki` 配下にまとめることで、OKF consumer は未知 key として保持でき、LLMWiki consumer は明示的に解釈できる。

## Consequences

- Positive: OKF compatibility と LLMWiki 固有制御を両立できる。
- Positive: 将来の key 追加範囲が明確になる。
- Negative: frontmatter が肥大化する場合は metadata store との分担が必要になる。

## Related Requirements

- [Requirement 009](../requirements/009-okf-compatible-format.md)
