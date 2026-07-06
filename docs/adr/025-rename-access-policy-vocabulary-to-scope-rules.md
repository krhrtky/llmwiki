---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 025: Rename Access Policy Vocabulary to Scope Rules

## Status

Accepted

## Context

[ADR 024](./024-delegate-store-edit-authorization-to-repository-controls.md) により、store の直接編集権限は repository controls に委譲した。これ以降、LLMWiki が扱う `query`、`related`、`export` などの operation-aware control は、repository authorization ではなく、選択済み store 内でどの知識を検索対象または出力対象に含めるかを決める責務に集中する。

しかし既存の `access_policy`、`policy object`、`allow` / `deny` といった語彙は、repository-level authorization と混同されやすい。特に `query` と `related` は検索対象範囲、`export` は出力対象範囲を決める入力であり、認可 policy というより scope selection rule として扱う方が実態に合う。

## Decision

CLI/API と SoT では、検索対象範囲や出力対象範囲を制御する語彙を次に置き換える。

- `query` と `related` の入力名は `retrieval_scope` とする。
- `export` の入力名は `export_scope` とする。
- 共通 schema 語彙は `scope rule` とし、`policy object` は使わない。
- `policy_id` は `rule_id` に置き換える。
- `policy_ids` は `rule_ids` に置き換える。
- `decision_logs` は `scope_evaluations` に置き換える。
- `decision` は `selection` に置き換える。
- `decided_by`、`decided_at` は `evaluated_by`、`evaluated_at` に置き換える。

この ADR の対象は CLI/API の対象範囲制御語彙に限り、redaction gate の `recommendation` (`allow` / `hold` / `deny`) は置き換え対象に含めない。

旧 CLI 語彙の互換 alias は作らず、docs と CLI 契約から即時に削除する。

`policy` は domain knowledge type としての文書種別や、組織ルール文書を指す通常名詞としては残してよい。ただし、CLI/API の対象範囲制御を指す語としては使わない。

## Alternatives

- `policy` を維持し、repository authorization との違いを説明で補う: 既存 docs の変更は少ないが、用語の誤読を恒久的に抱える。
- `filter` や `selector` に寄せる: 範囲選択の意味は出るが、operation-aware rule と監査記録のまとまりが弱くなる。
- `policy` を残しつつ alias を追加する: 移行は緩やかだが、CLI 契約が二重化し、即時削除方針に反する。

## Rationale

`retrieval_scope` と `export_scope` は、それぞれ「何を検索対象に含めるか」「何を出力対象に含めるか」を直感的に示す。`scope rule` と `scope evaluation` に寄せることで、repository write authorization と LLMWiki 内の operation-aware selection を明確に分離できる。

`include` / `exclude` / `hold` は、可否判定よりも範囲選択の結果として読みやすい。特に `query`、`related`、`export` の JSON output や YAML schema では、認可判定より selection result として解釈する方が混乱が少ない。一方で redaction gate は範囲選択ではなく提出可否の勧告であるため、`recommendation` は `allow` / `hold` / `deny` のまま維持する。

## Consequences

- Positive: repository authorization と LLMWiki の対象範囲制御の責務が分かれる。
- Positive: `query`、`related`、`export` の入力名が用途に対応する。
- Positive: 監査項目が `scope_evaluations`、`selection` として読める。
- Negative: ADR 005、ADR 016 を含む既存 docs の語彙更新が必要になる。
- Negative: 実装側も旧 CLI 語彙を即時に削除する必要がある。

## Related Requirements

- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
- [Requirement 015](../requirements/015-query-and-filing.md)

## Related ADRs

- [ADR 005](./005-use-operation-aware-access-control.md)
- [ADR 016](./016-finalize-access-policy-evaluation.md)
- [ADR 024](./024-delegate-store-edit-authorization-to-repository-controls.md)
