---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 020: Implementation Milestones

## Background

LLMWiki は format、workflow、access、graph、Agent Skill が絡む。実装順序を決めないと、重い基盤実装に先行して SoT が未整備になる。

## Milestones

### M1: SoT

- `docs/` の requirements、ADR、glossary、open questions を作る。
- 会話ログと参照元から判断材料を外化する。

### M2: Format

- OKF-compatible profile を定義する。
- frontmatter、reserved files、citation、index、log、link rule を固定する。

### M3: CLI/API

- `ingest`、`query`、`lint`、`graph`、`propose`、`redact`、`export` の最小 interface を定義する。
- file-first で実装できる範囲から開始する。

### M4: Workflow

- propose workflow、redaction gate、review、approval、rejection を実装する。
- access control は概念モデルから policy schema へ詳細化する。

### M5: Maintenance

- graph lint、docs lint、gardening Agent Skill を整備する。
- stale claim、orphan page、missing citation、contradiction を継続検出する。

## Acceptance Criteria

- 実装順序が M1 から M5 まで明示されている。
- 各 milestone が requirement と ADR に対応している。
- `model: gpt-5.4 medium` が実装フェーズの制約として記録されている。

## Related ADRs

- [ADR 001](../adr/001-use-docs-as-source-of-truth.md)
- [ADR 011](../adr/011-start-with-file-and-cli-first.md)
