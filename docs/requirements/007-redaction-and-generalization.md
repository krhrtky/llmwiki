---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 007: Redaction and Generalization

## Background

下位スコープには人事情報、顧客固有情報、契約情報、認証情報、未公開事業情報が含まれる可能性がある。org へ propose する際は、そのまま上位へ公開してはならない。

## Problem

秘匿情報が wiki の compile process に混入すると、org scope、graph、search、export、train によって影響範囲が拡大する。

## Goals

- propose 時に広めの秘匿基準で保護する。
- raw text copy を避け、抽象化・匿名化・要約を優先する。
- redaction report を残し、reviewer が判断できるようにする。

## Initial Sensitive Categories

- 人事情報。
- 個人情報。
- 顧客固有情報。
- 契約情報。
- 認証情報。
- 未公開事業情報。

## Acceptance Criteria

- propose workflow に redaction gate が必須である。
- 検出結果、変換方針、残リスクが report として残る。
- redaction できない場合は publish ではなく reject または hold になる。

## Related ADRs

- [ADR 004](../adr/004-require-redaction-gate.md)
- [ADR 005](../adr/005-use-operation-aware-access-control.md)
