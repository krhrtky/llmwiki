---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 005: Knowledge Lifecycle

## Background

LLMWiki の価値は、知識が追加されるたびに整理され、再利用され、矛盾や古さが検出されることにある。

## Problem

知識の状態遷移がない場合、発見途中の仮説、実務知識、公式ポリシー、廃止済み知識が混在する。

## Goals

- 知識の lifecycle を状態として扱う。
- propose、review、publish、deprecate を追跡できるようにする。
- stale claim と superseded knowledge を検出できるようにする。

## Lifecycle States

- `draft`: 作成中。
- `active`: scope 内で利用可能。
- `proposed`: 上位スコープへ提案中。
- `reviewing`: reviewer が確認中。
- `published`: 上位スコープで公開済み。
- `deprecated`: 廃止済み。
- `rejected`: 提案却下。

## Acceptance Criteria

- 状態遷移が propose workflow と review に接続されている。
- 廃止済み知識を削除ではなく追跡できる。
- org publish には review が必要と分かる。

## Related ADRs

- [ADR 003](../adr/003-use-propose-not-backport.md)
- [ADR 012](../adr/012-require-human-review-for-org-publish.md)
