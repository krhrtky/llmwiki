---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 016: Finalize Access Policy Evaluation

## Status

Accepted

## Context

ADR 005 で operation-aware access control を採用し、workflow/access 仕様で policy object と decision log の最小項目を固定した。ただし、複数 policy が同じ resource と operation に一致した場合の評価順序、優先順位、競合解決は未決のままだった。

この判断がないと、`query`、`propose`、`redaction_scan`、`export`、`train` の実装時に、同じ policy set から異なる decision が出る。特に `export` と `train` は高影響操作であり、誤って `allow` に倒すと下位 scope の情報が広がる。

## Decision

初期 access evaluator は、同じ request に一致した policy の decision を次の絶対優先順で解決する。

1. `deny`
2. `hold`
3. `allow`

`deny` が 1 件でも一致した場合、最終 decision は `deny` とする。`deny` がなく `hold` が 1 件以上一致した場合、最終 decision は `hold` とする。`allow` だけが一致した場合に限り `allow` とする。一致する policy がない場合は `hold` とする。

specificity は decision の優先順位を上書きしない。specificity は、同じ decision の policy が複数一致した場合に、decision log の説明対象 policy を選ぶために使う。specificity は次の順で高いものとする。

1. `resource.selector` が完全一致する policy
2. `resource.type` が一致する policy
3. `operation` が一致する policy
4. `content_level` が一致する policy
5. `scope` が一致する policy
6. `subject.id` が一致する policy
7. `subject.kind` または role が一致する policy

decision log は workflow/access 仕様の fixed minimum audit fields を必須とし、初期実装では追加監査項目を必須にしない。実装は任意で `matched_policy_ids`、`evaluation_reason`、`specificity_rank` を出力できるが、互換性の正本にはしない。

## Alternatives

- specificity を decision priority より優先する: 例外を表現しやすいが、広い `deny` を狭い `allow` が上書きでき、情報漏えい時の説明が難しくなる。
- 最初に一致した policy を採用する: 実装は単純だが、file order に security decision が依存する。
- `deny` と `allow` の競合を error にする: 厳密だが、policy 追加時に運用が止まりやすい。初期実装では `deny` に倒し、矛盾は lint または review で扱う。

## Rationale

LLMWiki の初期実装は file-first / CLI-first であり、policy の挙動を人間が JSON と sidecar から追える必要がある。`deny > hold > allow` は保守的で、`export` や `train` のような高影響操作を誤許可しにくい。

specificity を説明用に限定すると、例外表現の自由度は下がるが、初期 evaluator の挙動を deterministic にできる。例外許可が必要な場合は、広い `deny` を削除または narrowing する review を通す。

## Consequences

- Positive: policy conflict があっても deterministic に decision を返せる。
- Positive: 高影響操作で誤って `allow` に倒れにくい。
- Positive: decision log の最小項目だけで初期実装できる。
- Negative: 狭い例外 `allow` で広い `deny` を上書きできない。
- Negative: policy の矛盾検出や整理は別の lint / review task が必要になる。

## Related Requirements

- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 018](../requirements/018-review-and-ownership.md)

## Related ADRs

- [ADR 005](./005-use-operation-aware-access-control.md)
- [ADR 013](./013-finalize-m3-cli-contract.md)
