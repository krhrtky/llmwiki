# ADR 015: Store Typed Relations in LLMWiki Sidecar

## Status

Accepted

## Context

LLMWiki は Markdown link を graph edge の正本として扱う。一方で、owner、reviewer、risk_owner、claim ごとの relation のような補助 metadata は、本文とは別に page 隣接で保持しないと、graph lint と運用 metadata の責務が混ざる。

M3 では page 隣接 sidecar を採用済みであり、M5 では deterministic lint が `*.llmwiki.yaml` を入力に読む前提になっている。typed relation の保存場所をここで固定しないと、graph relation の入力元と lint の判定対象が分岐する。

## Decision

typed relation の補助 metadata は `page.llmwiki.yaml` に保存する。frontmatter、本文、`page.workflow.yaml` には保存しない。

`page.llmwiki.yaml` の最小 schema は次の項目とする。

- `owner`
- `reviewer`
- `risk_owner`
- `claims[]`
  - `claim_id`
  - `review_after`
  - optional `value`
- `relations[]`
  - `type`
  - `target`

Markdown link は graph edge の正本であり、typed relation は補助 metadata である。lint は top-level `relations[]` の `type` と `target` を読み、`graph.unknown_relation`、`graph.ambiguous_relation`、`graph.superseded_without_target`、`docs.contradiction` の入力として使う。

## Alternatives

- frontmatter に置く: page 本文と密結合しすぎて、schema が肥大化したときに保守しにくい。
- `page.workflow.yaml` に置く: workflow state と relation metadata の責務が混ざる。
- separate relation file にする: page、metadata、relation の参照が増え、file-first の単純さが落ちる。

## Rationale

`page.llmwiki.yaml` は page 隣接の運用 metadata を集める既存方針に合う。typed relation を同じ sidecar に置けば、owner や review 情報と同じ粒度で diff を確認できる。Markdown link を graph edge の正本として残すことで、本文側の構造と補助 metadata の役割分担も明確になる。

## Consequences

- Positive: relation metadata の保存場所が固定され、lint と graph builder の入力が揃う。
- Positive: page 本文を汚さずに typed relation を追記できる。
- Positive: owner、reviewer、risk_owner と relation metadata を同じ sidecar で review できる。
- Negative: `page.llmwiki.yaml` の schema 管理が必要になる。
- Negative: Markdown link と typed relation の二重管理が必要になるため、lint と編集運用で整合を保つ必要がある。

## Related Requirements

- [Requirement 011](../requirements/011-graph-and-relation.md)
- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
- [Requirement 016](../requirements/016-lint-and-gardening.md)
- [Requirement 020](../requirements/020-implementation-milestones.md)
