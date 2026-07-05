---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 015: Query and Filing

## Background

LLMWiki では質問への回答も知識として蓄積できる。比較、分析、接続、仮説は chat history に消えるのではなく wiki に戻せる。

## Problem

query 結果を保存しない場合、よい回答や発見が再利用されず、同じ探索を繰り返す。

## Goals

- query は wiki を読んで回答する。
- 回答が再利用価値を持つ場合は wiki page として filing する。
- filing する際は source、confidence、scope、owner を明示する。

## Acceptance Criteria

- query と filing が別操作として扱われる。
- filing には review または lint の対象になる metadata がある。
- 回答を無条件に org に公開しない。

## Related ADRs

- [ADR 010](../adr/010-use-index-and-log-for-progressive-disclosure.md)
- [ADR 005](../adr/005-use-operation-aware-access-control.md)
