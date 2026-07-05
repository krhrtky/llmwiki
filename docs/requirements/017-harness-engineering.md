---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 017: Harness Engineering

## Background

Harness Engineering では、Agent が信頼できる仕事をするために、repository-local な知識、検証、実行手順、ガードレールを整備する。

## Problem

AGENTS.md に全知識を詰め込むと、context を圧迫し、腐敗し、検証しにくい。Agent が読めない外部文書は実行時には存在しないのと同じになる。

## Goals

- AGENTS.md を詳細仕様ではなく map として扱う。
- `docs/` を SoT として progressive disclosure を実現する。
- lint、schema、test、review を通じて Agent の作業を検証可能にする。

## Acceptance Criteria

- `docs/` が SoT であることが明記されている。
- Agent が読む入口と深掘り先が分離されている。
- 将来、docs lint や graph lint を CI に載せられる。

## Related ADRs

- [ADR 001](../adr/001-use-docs-as-source-of-truth.md)
- [ADR 010](../adr/010-use-index-and-log-for-progressive-disclosure.md)
