# Requirement 011: Graph and Relation

## Background

LLMWiki は directory tree だけでなく、Markdown link による graph を持つ。concept、policy、procedure、decision、source が関係として辿れる必要がある。

## Problem

link がない wiki は page の集合に留まり、Agent が依存、制約、矛盾、根拠、廃止関係を判断しにくい。

## Goals

- page 間の link と relation を保守する。
- broken link、orphan page、重要概念の欠落を検出する。
- typed relation の扱いは後続 ADR に残す。

## Initial Relation Vocabulary

- `depends_on`
- `constrained_by`
- `implements`
- `specializes`
- `derived_from`
- `answers`
- `decided_by`
- `contradicts`
- `supersedes`
- `related_to`
- `example_of`
- `owned_by`
- `reviewed_by`

## Acceptance Criteria

- Markdown link が graph edge として扱われる。
- relation vocabulary の初期候補がある。
- typed relation の保存方式は未決事項として分離されている。

## Related ADRs

- [ADR 010](../adr/010-use-index-and-log-for-progressive-disclosure.md)
