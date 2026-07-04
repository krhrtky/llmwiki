# ADR 006: Adopt OKF-Compatible Markdown

## Status

Accepted

## Context

LLMWiki は Markdown wiki として運用する。独自形式を作るより、agent-readable な既存形式と互換にする方が tooling と移植性を得やすい。

## Decision

LLMWiki の wiki store は OKF v0.1 compatible な Markdown bundle とする。

## Alternatives

- 独自 JSON/YAML DB: 構造化しやすいが、人間の閲覧と編集がしにくい。
- Notion や Google Docs: 人間には便利だが、git diff、local lint、Agent の直接編集に弱い。
- OKF-compatible Markdown: 最小規約で人間と Agent の両方が扱える。

## Rationale

OKF v0.1 は Markdown file tree、YAML frontmatter、`type` 必須、`index.md`/`log.md` 予約という最小規約を定義している。LLMWiki の用途に十分近い。

## Consequences

- Positive: git、Obsidian、MkDocs、search、graph viewer と接続しやすい。
- Positive: producer-defined key により拡張できる。
- Negative: OKF v0.1 Draft の仕様変更を追跡する必要がある。

## Related Requirements

- [Requirement 009](../requirements/009-okf-compatible-format.md)

## References

- [Open Knowledge Format v0.1 SPEC](../references/index.md#open-knowledge-format-v01-spec)
