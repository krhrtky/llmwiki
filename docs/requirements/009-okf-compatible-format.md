# Requirement 009: OKF-Compatible Format

## Background

OKF v0.1 は、Markdown と YAML frontmatter による agent-readable な knowledge bundle format である。LLMWiki は独自 platform ではなく、OKF-compatible な bundle として保存する。

## Problem

独自形式を先に作ると、Obsidian、MkDocs、git、Agent、search、graph viewer など既存 tooling との互換性を失う。

## Goals

- OKF v0.1 の最小規約に従う。
- LLMWiki 固有情報は OKF を壊さない拡張として保持する。
- consumer が未知 key を許容できる設計にする。

## Format Requirements

- 非予約 `.md` は concept document として扱う。
- concept document は parseable YAML frontmatter を持つ。
- frontmatter は non-empty `type` を持つ。
- `index.md` と `log.md` は予約ファイルとして扱う。
- LLMWiki 固有メタデータは `llmwiki` 配下へ置く。

## Acceptance Criteria

- OKF v0.1 の conformance 条件を満たす。
- `llmwiki` 拡張が OKF の producer-defined key として扱える。
- OKF が platform ではなく format であることを前提にしている。

## Related ADRs

- [ADR 006](../adr/006-adopt-okf-compatible-markdown.md)
- [ADR 007](../adr/007-extend-okf-with-llmwiki-namespace.md)
