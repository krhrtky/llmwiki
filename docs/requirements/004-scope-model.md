---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 004: Storage Visibility Model

## Background

知識は private、team、org で可視性境界が異なる。全情報を中央に集約すると、秘匿性、所有責任、更新責任、ノイズの問題が発生する。

## Problem

policy だけに依存すると、CLI や Agent が誤った root を渡した場合に別の store を探索できてしまう。完全分散だけでは、共通語彙、横断ポリシー、組織判断が共有されない。

## Goals

- `private`、`team`、`org` を storage visibility boundary として扱う。
- `private` は最大 1 store とし、local path または repository を許可する。
- `team` は `team_id` ごとに複数 store を許可し、各 store は repository を 1 つ持つ。
- `org` は検討段階のため任意とし、設定する場合は最大 1 repository とする。
- 下位 store から上位 store への流れと、上位 store から下位 store への制約伝播を両立する。

## Storage Visibility Definitions

- `private`: 発見、仮説、個人メモ、private 相当の知識を置く物理境界。
- `team`: 実務に耐える再利用可能な局所知識を置く物理境界。`team_id` ごとに別 store とする。
- `org`: 組織横断の正規知識、語彙、制約、ポリシー、公式判断を置く物理境界。初期は任意。

`private` は page-level scope ではなく storage visibility boundary である。既存の `personal` metadata は private store への migration input として扱う。

## Acceptance Criteria

- `org` が任意であり、全文中央集約ではないことが明記されている。
- 各 storage visibility boundary の責務と管理主体が説明されている。
- 上位 store への移動が propose workflow に接続されている。
- `private` が page-level scope ではなく storage visibility boundary であることが明記されている。

## Related ADRs

- [ADR 023](../adr/023-use-storage-registry-for-visibility-boundaries.md)
- [ADR 003](../adr/003-use-propose-not-backport.md)
