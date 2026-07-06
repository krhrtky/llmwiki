---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 016: Finalize Scope Rule Evaluation

## Status

Accepted

## Context

ADR 005 で operation-aware access control を採用し、workflow/access 仕様で scope rule と scope evaluation の最小項目を固定した。ただし、複数 rule が同じ resource と operation に一致した場合の評価順序、優先順位、競合解決は未決のままだった。

この判断がないと、`query`、`propose`、`redaction_scan`、`export`、`train` の実装時に、同じ rule set から異なる selection が出る。特に `export` と `train` は高影響操作であり、誤って `include` に倒すと下位 scope の情報が広がる。

## Decision

初期 scope evaluator は、同じ request に一致した rule の selection を次の絶対優先順で解決する。

1. `exclude`
2. `hold`
3. `include`

`exclude` が 1 件でも一致した場合、最終 selection は `exclude` とする。`exclude` がなく `hold` が 1 件以上一致した場合、最終 selection は `hold` とする。`include` だけが一致した場合に限り `include` とする。一致する rule がない場合は `hold` とする。

specificity は selection の優先順位を上書きしない。specificity は、同じ selection の rule が複数一致した場合に、scope evaluation の説明対象 rule を選ぶために使う。specificity は次の順で高いものとする。

1. `resource.selector` が完全一致する rule
2. `resource.type` が一致する rule
3. `operation` が一致する rule
4. `content_level` が一致する rule
5. `scope` が一致する rule
6. `subject.id` が一致する rule
7. `subject.kind` または role が一致する rule

scope evaluation は workflow/access 仕様の fixed minimum audit fields を必須とし、初期実装では追加監査項目を必須にしない。実装は任意で `matched_rule_ids`、`evaluation_reason`、`specificity_rank` を出力できるが、互換性の正本にはしない。

## Alternatives

- specificity を selection priority より優先する: 例外を表現しやすいが、広い `exclude` を狭い `include` が上書きでき、情報漏えい時の説明が難しくなる。
- 最初に一致した rule を採用する: 実装は単純だが、file order に selection が依存する。
- `exclude` と `include` の競合を error にする: 厳密だが、rule 追加時に運用が止まりやすい。初期実装では `exclude` に倒し、矛盾は lint または review で扱う。

## Rationale

LLMWiki の初期実装は file-first / CLI-first であり、scope rule の挙動を人間が JSON と sidecar から追える必要がある。`exclude > hold > include` は保守的で、`export` や `train` のような高影響操作を誤って出力対象へ含めにくい。

specificity を説明用に限定すると、例外表現の自由度は下がるが、初期 evaluator の挙動を deterministic にできる。例外許可が必要な場合は、広い `exclude` を削除または narrowing する review を通す。

## Consequences

- Positive: rule conflict があっても deterministic に selection を返せる。
- Positive: 高影響操作で誤って `include` に倒れにくい。
- Positive: scope evaluation の最小項目だけで初期実装できる。
- Negative: 狭い例外 `include` で広い `exclude` を上書きできない。
- Negative: rule の矛盾検出や整理は別の lint / review task が必要になる。

## Related Requirements

- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 018](../requirements/018-review-and-ownership.md)

## Related ADRs

- [ADR 005](./005-use-operation-aware-access-control.md)
- [ADR 013](./013-finalize-m3-cli-contract.md)
- [ADR 024](./024-delegate-store-edit-authorization-to-repository-controls.md)
- [ADR 025](./025-rename-access-policy-vocabulary-to-scope-rules.md)
