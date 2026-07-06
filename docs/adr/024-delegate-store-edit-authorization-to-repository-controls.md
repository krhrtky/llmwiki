---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 024: Delegate Store Edit Authorization to Repository Controls

## Status

Accepted

## Context

[ADR 023](./023-use-storage-registry-for-visibility-boundaries.md) で、`private`、`team:<team_id>`、任意の `org` store の物理的な visibility boundary として storage registry を採用した。各 layer は別 repository または明確な repository root で管理できる。

この変更により、write authorization の責務分離を決める必要がある。LLMWiki も layer ごとの編集可否を判定すると、repository permission、branch protection、CODEOWNERS、review rule と LLMWiki scope rule が同じ判断を重複して持つ。重複した authorization は drift を生み、ある system では許可され、別 system では拒否される状態を作る。

`query`、`related`、`propose`、`export`、`train` などの操作には operation-aware access control が必要である。一方、layer の直接編集可否は repository boundary で enforcement する方が責務が明確になる。

## Decision

LLMWiki は store の直接編集権限を、各 store を host または管理する repository に委譲する。

LLMWiki は、subject が選択された `private`、`team:<team_id>`、`org` store を直接編集できるかについて、独自の scope evaluation を持たない。直接編集の source of truth は、write permission、branch protection、CODEOWNERS、required review、merge permission、repository audit log などの repository-level controls とする。

LLMWiki は次の責務を持つ。

- storage registry で `--store` を解決し、選択 store root の外側の path を拒否する。
- LLMWiki operation から raw source の immutable 境界を守る。
- layer 間で暗黙に publish せず、proposal、redaction、review、filing artifact を作る。
- `read`、`search`、`retrieve`、`query`、`answer_suggestion`、`related`、`propose`、`redaction_scan`、`generalize`、`lint`、`graph_build`、`export`、`publish`、`train` などの read / derived operation に operation-aware scope rule を適用する。
- `include`、`hold`、`exclude` を返す LLMWiki operation の scope evaluation を記録する。

この ADR は [ADR 005](./005-use-operation-aware-access-control.md) と [ADR 016](./016-finalize-access-policy-evaluation.md) を狭める。operation-aware scope rule は引き続き採用するが、store の直接編集に対する repository write authorization は model 化しない。

## Alternatives

- LLMWiki scope rule で編集権限も管理する: すべての判断を 1 つの schema に集約できるが、repository authorization と重複し、branch protection や CODEOWNERS の authority が曖昧になる。
- repository authorization だけを使い、operation-aware scope rule を全て削除する: LLMWiki は単純になるが、`metadata` visibility、`export` restriction、`train` exclusion、`propose` redaction requirement などの content-level / operation-level constraint を表現できない。
- repository authorization を LLMWiki scope rule の input として扱う: LLMWiki 側に scope evaluation を集約できるが、repository host ごとの adapter が必要になり、file-first CLI が外部 authorization API に依存する。

## Rationale

Repository system は layer boundary における direct write access をすでに enforce している。branch rule、required review、protected branch、owner review、merge permission、audit log は LLMWiki が再実装すべきではない。

LLMWiki scope rule の価値は別にある。知識をどのように read、transform、export、train、store 間 promote するかを制御することである。これらの operation は direct repository edit ではなくても知識を漏えいまたは拡散しうるため、operation-aware selection と log が必要である。

この分離により、storage visibility、repository edit authorization、LLMWiki operation scope rule の責務を分ける。

## Consequences

- Positive: direct edit authorization の source of truth が store repository ごとに 1 つになる。
- Positive: LLMWiki scope rule schema を operation と content-level constraint に集中できる。
- Positive: store layer ownership は custom LLMWiki adapter なしで既存の repository governance を使える。
- Negative: repository host が理由を公開しない限り、LLMWiki は repository write denial の理由を説明できない。
- Negative: local filesystem store は LLMWiki scope evaluator ではなく filesystem permission と human process に依存する。
- Negative: 実装は、LLMWiki validation の成功が repository write permission を与えるものではないことを明記する必要がある。

## Open Questions

- repository-host-specific audit ingestion は、LLMWiki report に repository permission decision を表示する必要が出た場合、後続の operations design で扱う。

## Related Requirements

- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
- [Requirement 018](../requirements/018-review-and-ownership.md)

## Related ADRs

- [ADR 005](./005-use-operation-aware-access-control.md)
- [ADR 016](./016-finalize-access-policy-evaluation.md)
- [ADR 023](./023-use-storage-registry-for-visibility-boundaries.md)
- [ADR 025](./025-rename-access-policy-vocabulary-to-scope-rules.md)
