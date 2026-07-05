---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 013: API and CLI

## Background

LLMWiki Core はドメインアプリから独立した知識基盤として利用する。Agent は shell や API から同じ操作を実行できる必要がある。

## Problem

操作が手作業や特定 UI に依存すると、Agent が再現可能に ingest、query、lint、propose を実行できない。

## Goals

- 初期 CLI/API 操作を定義する。
- file-first で始め、必要になるまで DB や vector DB を必須にしない。
- Agent が標準 tooling から呼び出せる形にする。

## Initial Operations

- `ingest`: raw source から wiki 更新候補を作る。
- `query`: wiki を検索し、回答を生成する。
- `lint`: wiki の矛盾、古さ、欠落、link を検査する。
- `graph`: link graph を構築する。
- `propose`: 上位 scope への proposal を作る。
- `redact`: 秘匿情報を検出し、redaction report を作る。
- `export`: bundle または scope を出力する。

## Acceptance Criteria

- CLI/API の初期操作が requirements と対応している。
- domain application の case や customer 管理と混同していない。
- 実装詳細は後続設計に残されている。

## Related ADRs

- [ADR 008](../adr/008-separate-core-from-domain-apps.md)
- [ADR 011](../adr/011-start-with-file-and-cli-first.md)
