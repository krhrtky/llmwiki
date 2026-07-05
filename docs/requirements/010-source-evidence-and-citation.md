---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 010: Source Evidence and Citation

## Background

wiki は raw source の解釈であり、source of truth そのものではない。事実 claim は根拠を辿れる必要がある。

## Problem

根拠のない claim が wiki に混入すると、Agent はそれを事実として再利用し、誤りが compounding する。

## Goals

- raw source と wiki page を分離する。
- claim には source または confidence を持たせる。
- citation を Markdown で辿れるようにする。

## Evidence Rules

- raw source は immutable とする。
- wiki page は source への citation を持つ。
- source がない claim は confidence を下げる。
- org policy は source、decision、reviewer を必須にする。

## Acceptance Criteria

- source と wiki の役割が分離されている。
- `# Citations` または equivalent な citation 表現を使う。
- 根拠不明 claim を lint 対象にできる。

## Related ADRs

- [ADR 009](../adr/009-keep-raw-sources-immutable.md)
- [ADR 006](../adr/006-adopt-okf-compatible-markdown.md)
