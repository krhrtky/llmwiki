---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 008: Operation-Aware Access Control

## Background

単純な `visibility: team` では、検索には出してよいが本文取得は不可、graph には edge だけ載せる、train には使わない、といった制御を表現できない。

## Problem

操作ごとの参照可否を分けない場合、低リスク操作に許可した情報が高リスク操作へ流用される。

## Goals

- access control を operation-aware にする。
- 初期 docs では concept model と scope rule の最小契約項目まで固定し、実装詳細は後続 ADR に送る。
- content level を `metadata`、`summary`、`content` に分ける。

## Operations

- `read`
- `search`
- `retrieve`
- `query`
- `answer_suggestion`
- `propose`
- `redaction_scan`
- `generalize`
- `lint`
- `graph_build`
- `export`
- `publish`
- `train`

## Concept Model

- `subject`: user、agent、service account。
- `scope`: personal、team、org。
- `store_id`: `private`、`team:<team_id>`、`org` のいずれか。
- `team_id`: team store の場合の team 識別子。
- `operation`: 実行しようとしている操作。
- `content_level`: metadata、summary、content。
- `selection`: include、exclude、hold。
- `scope_evaluation`: 誰が、何を、どの理由で対象範囲に含めたか、除外したか、保留したか。

## Fixed Minimum Contract

- `scope rule` は `subject`、`scope`、`store_id`、`team_id`、`operation`、`content_level`、`resource`、`selection`、`reason`、`conditions` を持てる。
- `store_id` と `team_id` は任意だが、指定された場合は request と一致しなければ rule は一致しない。
- `scope_evaluation` は `subject`、`operation`、`content_level`、`store_id`、`team_id`、`resource`、`selection`、`rule_ids`、`evaluated_by`、`evaluated_at`、`reason` を持てる。

## Acceptance Criteria

- visibility だけでは不十分である理由が説明されている。
- operation と content level が分離されている。
- 複数 team store で scope rule が誤適用されないように `store_id` と `team_id` を評価できる。
- 最小契約項目と、後続 ADR に残す評価順序が区別されている。

## Related ADRs

- [ADR 005](../adr/005-use-operation-aware-access-control.md)
