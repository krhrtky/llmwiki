---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 020: Use Hybrid Search and Graph Traversal for Related Retrieval

## Status

Accepted

## Context

単純な grep は、同義語、逆参照、制約関係、矛盾、置換、operation-aware access control を扱えない。LLMWiki は Markdown link と typed relation を持つため、検索は文字列一致だけでなく relation を辿る retrieval として拡張する必要がある。

一方で、ADR 011 は file-first / CLI-first を採用しており、DB、vector DB、外部 service を初期必須にしない。PostgreSQL、pgvector、OpenSearch、Neo4j、GraphRAG は有力な後続 index adapter だが、現時点で SoT の正本にしてはならない。

## Decision

related retrieval は次の責務分離で設計する。

- Search: lexical search と将来の vector search で `llmwiki retrieve` の seed page または section を見つける。
- Graph traversal: Markdown link と `*.llmwiki.yaml` の typed relation から derived graph index を作り、seed から relation を辿る。
- Access filter: seed、edge、neighbor、section body の各段階で operation-aware access control を適用する。
- Rerank / explain: 最終 context を選び、なぜ取得されたかを path と scope evaluation で説明する。

初期実装は file-first derived index を使う。DB、pgvector、OpenSearch、Neo4j、GraphRAG は後続 adapter とし、Markdown / sidecar / raw source の SoT を置き換えない。

Post-M5 の最初の固定 CLI/API は `llmwiki related` とする。`related` は明示 seed を入力に取り、関連展開と説明可能性を担う。`llmwiki retrieve` は lexical seed selection、section chunk、BM25、embedding を要する後続 command として残す。

## Alternatives

- grep / ripgrep のみを使う: 実装は単純だが、関係探索、逆参照、同義語、access-aware retrieval を扱えない。
- 最初から PostgreSQL + pgvector を必須にする: 実用的だが、ADR 011 の file-first 境界より infra 前提が強くなる。
- 最初から Neo4j を正本にする: graph query は強いが、Markdown bundle を SoT にする方針と同期設計が複雑になる。
- GraphRAG を中核検索にする: global summary には有効だが、手順回答や制約取得には過剰で、access control と redaction gate の設計負荷が高い。

## Rationale

検索入口、関連展開、アクセス制御、最終選別を分けると、LLMWiki の既存要件と矛盾せずに拡張できる。typed relation は依存、制約、根拠、矛盾、置換を明示し、hybrid search は seed の recall を補う。

GraphRAG は通常 retrieval の置き換えではなく、org-level sensemaking、propose candidate discovery、横断矛盾の発見に使う。

## Consequences

- Positive: grep の限界を超え、関連 policy、ADR、source、FAQ を取得できる。
- Positive: relation path と scope evaluation を返すことで、LLM が使った知識の説明可能性が上がる。
- Positive: `related` を先に固定することで、file-first を維持したまま最小の retrieval surface で実装を始められる。
- Positive: file-first を維持したまま、PostgreSQL / pgvector / search index / graph DB へ拡張できる。
- Positive: `retrieve` の lexical seed selection、section chunk、BM25、embedding は後続の独立した設計課題として切り出せる。
- Negative: relation vocabulary、traversal rule、score、access filter の仕様が必要になる。
- Negative: vector search や LLM relation extraction は deterministic lint と別の品質管理が必要になる。

## Related Requirements

- [Requirement 011](../requirements/011-graph-and-relation.md)
- [Requirement 015](../requirements/015-query-and-filing.md)
- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 014](../requirements/014-storage-boundary.md)

## Related ADRs

- [ADR 011](./011-start-with-file-and-cli-first.md)
- [ADR 015](./015-store-typed-relations-in-llmwiki-sidecar.md)
- [ADR 016](./016-finalize-access-policy-evaluation.md)
- [ADR 021](./021-trace-docs-and-implementation-with-stable-evidence-links.md)
