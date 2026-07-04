# ADR 014: Finalize M5 Maintenance Contract

## Status

Accepted

## Context

M5 Maintenance は、graph lint、docs lint、gardening Agent Skill を継続運用に載せる段階である。ここで検出対象を曖昧にすると、本文意味比較や LLM 推論、外部 DLP 連携が先に混ざり、file-first / CLI-first の実装と CI gate の再現性が崩れる。

published page の citation 不足は、既に公開された知識の根拠欠落として扱う必要がある。

## Decision

M5 の初期 maintenance は deterministic lint を正本にする。lint report の正式 output contract は JSON とし、Markdown report は JSON から派生生成する表示に留める。

初期対象は structured metadata と明示 relation のみに限定する。具体的には、claim / stale / contradiction の初期検出は frontmatter や sidecar にある構造化 metadata と明示 relation を入力にし、本文の意味比較や LLM 推論には依存しない。`## Citations` と段落末尾の Markdown citation link は citation 検査の入力であり、claim 抽出の入力ではない。DLP / redaction scan の実装方式はこの ADR では固定せず、後続判断に残す。

published page に missing citation がある場合は CI error とする。published でない page の missing citation は、初期の CI gate では hard fail にしない。

## Alternatives

- 本文の LLM 比較まで初期実装する: implicit contradiction や stale を広く拾えるが、再現性が低く、誤検出で CI を止めやすい。
- claim 系の検出をすべて後回しにする: 実装は単純になるが、M5 に期待される継続検出の入口がなくなる。
- CI gate を citation なしで最小化する: 導入は速いが、published page の evidence 不足を見逃す。
- DLP / redaction scan まで初期に固定する: 外部依存と運用境界が増え、file-first / CLI-first の実装を鈍らせる。

## Rationale

deterministic lint は file-first repository で再現しやすく、CLI automation の挙動を予測可能にする。JSON を正式 contract にすると、Agent と shell が同じ report を扱える。M5 の初期範囲を structured metadata と明示 relation に絞ることで、抽出の問題と検出の問題を混ぜずに済む。published page の citation 不足は明確な evidence failure なので、CI error にするのが妥当である。

## Consequences

- Positive: 誤検出を抑えやすく、CI の再現性が高い。
- Positive: file-first / CLI-first の実装にそのまま載せやすい。
- Positive: JSON contract により Agent と shell で同じ report を扱える。
- Positive: published page の evidence 不足を hard fail できる。
- Negative: 暗黙矛盾や暗黙 stale claim は初期段階では検出できない。
- Negative: DLP / redaction scan と本文意味比較は別判断として後続化される。

## Open Questions

- typed relation schema の保存場所。
- redaction scan の実装方式。
- 本文意味比較による contradiction / stale 検出の採用可否と実装方式。
- source 更新に基づく stale claim 検出の metadata contract。

これらは [未決事項](../open-questions.md) に集約する。

## Related Requirements

- [Requirement 010](../requirements/010-source-evidence-and-citation.md)
- [Requirement 011](../requirements/011-graph-and-relation.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
- [Requirement 015](../requirements/015-query-and-filing.md)
- [Requirement 016](../requirements/016-lint-and-gardening.md)
- [Requirement 017](../requirements/017-harness-engineering.md)
- [Requirement 020](../requirements/020-implementation-milestones.md)
