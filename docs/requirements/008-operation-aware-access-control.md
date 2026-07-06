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
- 初期 docs では concept model と最小契約項目まで固定し、認可エンジンの実装詳細は後続 ADR に送る。
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
- `decision`: allow、deny、hold。
- `decision_log`: 誰が、何を、どの理由で許可または拒否したか。

## Fixed Minimum Contract

- `policy object` は `subject`、`scope`、`store_id`、`team_id`、`operation`、`content_level`、`resource`、`decision`、`reason`、`conditions` を持てる。
- `store_id` と `team_id` は任意だが、指定された場合は request と一致しなければ policy は一致しない。
- `decision_log` は `subject`、`operation`、`content_level`、`store_id`、`team_id`、`resource`、`decision`、`policy_ids`、`decided_by`、`decided_at`、`reason` を持てる。

## Acceptance Criteria

- visibility だけでは不十分である理由が説明されている。
- operation と content level が分離されている。
- 複数 team store で policy が誤適用されないように `store_id` と `team_id` を評価できる。
- 最小契約項目と、後続 ADR に残す評価順序が区別されている。

## Related ADRs

- [ADR 005](../adr/005-use-operation-aware-access-control.md)
