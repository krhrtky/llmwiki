---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 006: Propose Workflow

## Background

会話ログでは、上位 store への移動操作名として `propose` を採用した。これは下位 store の知識をそのまま copy する操作ではない。

## Problem

backport や copy という表現は、下位 store の文脈や秘匿情報をそのまま上位へ移す誤解を生む。

## Goals

- `private → team:<team_id>`、`team:<team_id> → org` の昇格候補を propose として扱う。
- `org` store は任意であり、未設定時は `team:<team_id> → org` を実行しない。
- propose 時に根拠、抽象化、匿名化、差分、reviewer を明示する。
- propose の拒否理由を記録し、再提案できるようにする。

## Workflow

1. 下位 store の page または page 群を選ぶ。
2. propose draft を作る。
3. sensitivity scan を実行する。
4. redaction / generalization を行う。
5. evidence と source link を整理する。
6. reviewer と approver を割り当てる。
7. review で approve または reject する。
8. approve された draft を上位 store に publish する。
9. 元 page と publish page を link する。

## Acceptance Criteria

- propose が copy/backport ではないと明記されている。
- redaction gate と human review に接続されている。
- reject の記録と再提案が可能である。

## Related ADRs

- [ADR 003](../adr/003-use-propose-not-backport.md)
- [ADR 004](../adr/004-require-redaction-gate.md)
- [ADR 012](../adr/012-require-human-review-for-org-publish.md)
- [ADR 023](../adr/023-use-storage-registry-for-visibility-boundaries.md)
