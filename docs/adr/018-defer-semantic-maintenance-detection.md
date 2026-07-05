---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 018: Defer Semantic Maintenance Detection

## Status

Accepted

## Context

ADR 014 で M5 の初期 maintenance は deterministic lint に固定し、structured metadata と明示 relation を入力にする方針を採用した。一方で、本文意味比較による contradiction / stale 検出と、source 更新に基づく stale claim 検出の metadata contract は未決のままだった。

これらは有用だが、LLM 推論、embedding、source revision model、snapshot id などの設計が必要になる。完全完成の blocker として扱うと、M3 / M4 / M5 の初期 command と workflow 実装が止まる。

## Decision

本文意味比較による contradiction / stale 検出は、LLMWiki Core 初期完成の必須範囲に含めない。source 更新に基づく stale claim 検出も、初期完成の必須範囲に含めない。

初期完成の maintenance 判定は、ADR 014 の deterministic lint に限定する。

- `review_after` の期限超過による `docs.stale_claim`
- explicit `contradicts` relation
- 同一 `claim_id` の structured metadata 不一致
- published page の citation 検査
- graph / docs lint target

semantic contradiction / stale detection と source-update stale detection は、後続 enhancement として扱う。これらを実装する場合は、先に source revision metadata、snapshot id、比較対象、LLM/embedding の使用可否、誤検出時の workflow を別 ADR または design doc で固定する。

## Alternatives

- 初期完成に semantic detection を含める: 暗黙矛盾を拾えるが、再現性と評価基準が不足する。
- source 更新 stale detection だけ初期に含める: 実用性はあるが、source revision metadata contract が未定義のため、raw source immutable 境界と衝突しやすい。
- すべての stale / contradiction を後回しにする: 実装は軽いが、M5 の継続検出として弱すぎる。

## Rationale

初期完成では、CLI と file-first store で再現できる検出に限定する方が、実装と CI gate を安定させやすい。structured metadata と明示 relation に限定しても、owner / reviewer / stale / contradiction / citation / graph の初期 maintenance loop は成立する。

semantic detection は、検出精度だけでなく、誰が採否を判断するか、どの metadata を正本にするか、誤検出をどう扱うかが重要である。これは初期完成後の運用 feedback を踏まえて設計する。

## Consequences

- Positive: 初期完成の scope が deterministic lint に閉じる。
- Positive: M3 / M4 / M5 の残実装に進める。
- Positive: source revision metadata を急いで決めずに済む。
- Negative: 暗黙矛盾や source 更新由来の stale は初期段階では検出できない。
- Negative: semantic maintenance は後続 roadmap で改めて優先順位付けが必要になる。

## Related Requirements

- [Requirement 010](../requirements/010-source-evidence-and-citation.md)
- [Requirement 016](../requirements/016-lint-and-gardening.md)
- [Requirement 020](../requirements/020-implementation-milestones.md)

## Related ADRs

- [ADR 014](./014-finalize-m5-maintenance-contract.md)
