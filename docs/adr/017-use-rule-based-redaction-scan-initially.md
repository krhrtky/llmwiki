---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 017: Use Rule-Based Redaction Scan Initially

## Status

Accepted

## Context

ADR 004 で `propose` に redaction / generalization gate を必須化した。workflow/access 仕様では redaction gate の出力契約を固定したが、検出方式、sanitized draft の保存場所、residual risk の扱いは未決だった。

この判断がないと、`redact` と `propose` の実装で、外部 DLP、LLM、rule-based scan のどれを前提にするかが分岐する。初期実装は file-first / CLI-first を維持し、ローカルで再現できる必要がある。

## Decision

初期 redaction scan は rule-based deterministic scan とする。LLM、外部 DLP service、外部 policy engine は初期実装の必須依存にしない。

初期 scanner は、次の category を rule set として扱う。

- `personal_data`
- `customer_specific`
- `contract`
- `credential`
- `unpublished_business`
- `hr`

`redact` command は raw source と wiki 本文を上書きしない。redaction report と sanitized draft は workspace 管理下の `.llmwiki/redactions/` に保存する。

保存 artifact は次の 2 種類に分ける。

- `redaction_report`: findings、transformations、residual_risk、blocked_items、recommendation、source_paths、target_scope。
- `sanitized_draft`: redaction / generalization 後の draft content または draft file reference。

`recommendation` は `allow`、`hold`、`deny` の 3 値とする。

- findings がなく、blocked_items と residual_risk が空の場合は `allow`。
- findings があり、rule-based transformation で置換可能だが reviewer 判断が必要な場合は `hold`。
- credential や raw text copy など、上位 scope へ出せない要素が残る場合は `deny`。

residual_risk が空でない proposal は、`risk_owner` が割り当てられるまで `hold` とする。`risk_owner` が割り当てられても、初期実装では自動 publish しない。`propose` は report 参照を draft に含め、reviewer / approver の human review に渡す。

## Alternatives

- LLM scan を初期採用する: 文脈を拾いやすいが、再現性、監査性、実行環境依存が増える。
- 外部 DLP service を初期採用する: 検出品質は期待できるが、file-first / CLI-first の前提と外部 service 非必須の境界を崩す。
- hybrid を初期採用する: 将来性はあるが、初期 contract と failure mode が複雑になる。
- reviewer の目視だけにする: 実装は軽いが、ADR 004 の redaction gate 必須化を満たさない。

## Rationale

rule-based scan はローカルで再現でき、CI や CLI smoke に載せやすい。検出範囲は狭いが、credential や明示的な個人情報 pattern など、初期に止めるべき risk を deterministic に扱える。

`.llmwiki/redactions/` に report と draft を置くことで、page 本文、raw source、page sidecar、workflow sidecar を汚さずに redaction の派生成果物を管理できる。workflow sidecar には report 参照と review decision を記録し、report 本体は derived artifact として扱う。

## Consequences

- Positive: redaction scan を外部依存なしで実装できる。
- Positive: `redact` と `propose` の artifact 保存先が固定される。
- Positive: residual risk が human review に接続される。
- Negative: 文脈依存の秘匿情報は初期 rule では検出できない。
- Negative: false positive / false negative は rule maintenance で扱う必要がある。
- Negative: LLM / DLP 連携は後続拡張として別途 ADR が必要になる。

## Open Questions

- rule set の具体 pattern と severity は implementation design で扱う。
- LLM または DLP service を追加する条件は、初期 rule-based scanner の運用後に後続 ADR とする。

## Related Requirements

- [Requirement 006](../requirements/006-propose-workflow.md)
- [Requirement 007](../requirements/007-redaction-and-generalization.md)
- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 014](../requirements/014-storage-boundary.md)

## Related ADRs

- [ADR 004](./004-require-redaction-gate.md)
- [ADR 005](./005-use-operation-aware-access-control.md)
- [ADR 011](./011-start-with-file-and-cli-first.md)
- [ADR 013](./013-finalize-m3-cli-contract.md)
