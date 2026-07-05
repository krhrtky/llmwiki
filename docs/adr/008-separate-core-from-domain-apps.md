---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 008: Separate Core from Domain Applications

## Status

Accepted

## Context

LLMWiki は問い合わせ管理などの domain application と接続できるが、それ自体は知識基盤である。case、customer、assignment、SLA などを Core に含めると責務が混ざる。

## Decision

LLMWiki Core と domain application を分離する。

## Alternatives

- domain application に組み込む: 初期連携は速いが、他用途に再利用しにくい。
- LLMWiki Core を独立させる: 境界設計が必要だが、知識基盤として再利用できる。

## Rationale

LLMWiki は「何を根拠に、どう判断すべきか」を扱う。問い合わせ管理は「誰のどの問い合わせに、いつ誰がどう対応するか」を扱う。

## Consequences

- Positive: Core を個人 wiki、チーム wiki、問い合わせ管理、研究 wiki に再利用できる。
- Positive: domain data と knowledge data の権限境界を分けられる。
- Negative: integration API が必要になる。

## Related Requirements

- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
