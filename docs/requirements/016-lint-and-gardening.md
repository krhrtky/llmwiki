---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 016: Lint and Gardening

## Background

LLMWiki は成長するほど stale claim、orphan page、broken link、矛盾、重複が増える。継続保守のために lint と gardening が必要である。

## Problem

保守されない wiki は古くなり、Agent が誤った前提を再利用する。人間だけで gardening すると運用負荷が高い。

## Goals

- lint で機械的に検出できる問題を定義する。
- gardening を Agent の定期タスクにできるようにする。
- 人間判断が必要な問題は open question または proposal に分離する。

## Lint Targets

- broken link。
- orphan page。
- missing citation。
- stale claim。
- duplicated concept。
- contradiction。
- missing owner。
- missing reviewer for org policy。
- unknown or invalid lifecycle state。

## Acceptance Criteria

- lint target が列挙されている。
- lint は修正ではなく検出と提案を基本にする。
- 自動修正してはいけない問題を人間判断に回せる。

## Related ADRs

- [ADR 001](../adr/001-use-docs-as-source-of-truth.md)
- [ADR 010](../adr/010-use-index-and-log-for-progressive-disclosure.md)
