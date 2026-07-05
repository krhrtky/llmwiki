---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 004: Scope Model

## Background

知識は個人、チーム、組織横断で有効範囲が異なる。全情報を中央に集約すると、秘匿性、所有責任、更新責任、ノイズの問題が発生する。

## Problem

中央集権だけでは、チーム固有の知識や未成熟な仮説まで org に混入する。完全分散だけでは、共通語彙、横断ポリシー、組織判断が共有されない。

## Goals

- `personal → team → org` の 3 層モデルを採用する。
- `private` は独立 scope として追加せず、個人/private 相当の知識は `personal` で扱う。
- 知識本文は分散可能にし、識別子・関係・制約は横断的に扱えるようにする。
- 下位から上位への流れと、上位から下位への制約伝播を両立する。

## Scope Definitions

- `personal`: 発見、仮説、個人メモ。
- `team`: 実務に耐える再利用可能な局所知識。
- `org`: 組織横断の正規知識、語彙、制約、ポリシー、公式判断。

`private` は scope ではなく、`personal` 内の秘匿性または access policy で表現する。scope は知識の有効範囲を表し、秘匿度、公開可否、operation ごとの参照可否は `llmwiki.lifecycle`、sidecar metadata、operation-aware access policy で扱う。

## Acceptance Criteria

- `org` が全文中央集約ではないことが明記されている。
- 各 scope の責務と管理主体が説明されている。
- 上位スコープへの移動が propose workflow に接続されている。
- `private` を新 scope とせず、個人/private 相当を `personal` と access policy で扱うことが明記されている。

## Related ADRs

- [ADR 002](../adr/002-adopt-personal-team-org-scope.md)
- [ADR 003](../adr/003-use-propose-not-backport.md)
