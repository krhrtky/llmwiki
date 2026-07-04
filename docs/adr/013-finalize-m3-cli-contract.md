# ADR 013: Finalize M3 CLI Contract

## Status

Accepted

## Context

M3 は `ingest`、`query`、`file`、`lint`、`graph`、`propose`、`redact`、`export` の最小 interface を実装に落とす段階である。ここで transport と storage layout を曖昧にすると、CLI、Agent、将来の adapter で contract が分岐し、file-first の前提も崩れる。

[ADR 007](./007-extend-okf-with-llmwiki-namespace.md) は、LLMWiki 固有 metadata を frontmatter の `llmwiki` namespace に置く方針を採用している。一方で、その ADR 自身が frontmatter 肥大化時の metadata store 分担を将来課題として認めている。M3 ではこの分担ルールも固定する必要がある。

## Decision

M3 の CLI は単一 binary `llmwiki` とする。CLI は `llmwiki <command>` 形式で `ingest`、`query`、`file`、`lint`、`graph`、`propose`、`redact`、`export` を公開し、同じ意味の操作は内部関数 API を正本として共有する。

M3 では HTTP API を必須にしない。machine-readable output の正式 transport は JSON のみに固定し、Markdown report は JSON から生成する派生表示として扱う。

page-level の metadata は 2 層に分ける。`llmwiki.scope`、`llmwiki.lifecycle` など page とともに運ぶ正本 metadata は frontmatter の `llmwiki` namespace に置く。owner、reviewer、risk_owner、confidence、citation metadata、access policy refs など肥大化しやすい運用 metadata は `page.llmwiki.yaml` に置く。proposal、review、approval、rejection、hold reason などの workflow state は `page.workflow.yaml` に置く。`page.md` の本文と sidecar は隣接して管理する。

この ADR は [ADR 007](./007-extend-okf-with-llmwiki-namespace.md) を破棄しないが、frontmatter と sidecar の責務分担について具体化する。

## Alternatives

- CLI と HTTP API を同時に立てる: network、auth、deployment の境界が増え、M3 の file-first 実装を遅らせる。
- 複数 binary や command ごとの個別実行ファイルに分ける: Agent からの呼び出し経路が分散し、同じ意味の操作を揃えにくい。
- 人間向けの text output を正式 transport にする: 手で読むには便利だが、自動化と検証の contract として弱い。
- metadata や workflow state を page frontmatter や中央集約 store に寄せる: page 本文の責務と mutable state が混ざり、file-first と diffability が落ちる。

## Rationale

単一 binary は人間と Agent の両方にとって入口が明確である。内部関数 API を正本にすると、CLI と将来の adapter が同じ意味論を共有できるが、M3 では HTTP のような余計な transport を固定しなくてよい。

JSON only の output contract は、shell、test、Agent のいずれからも同じ形式で扱える。Markdown report を派生表示に限定すると、表示の自由度を残しつつ機械可読 contract を壊さない。

page-adjacent sidecar は、page 本文を immutable に保ちながら metadata と workflow state を diffable なファイルとして分離できる。frontmatter に `llmwiki` namespace を残すことで ADR 007 の方針も維持できる。これは `docs/` の file-first 方針と storage boundary の要求に合う。

## Consequences

- Positive: CLI、Agent、将来の adapter が同じ operation contract を共有できる。
- Positive: JSON output により検証と自動化が安定する。
- Positive: `page.llmwiki.yaml` と `page.workflow.yaml` が review 可能な差分として残る。
- Positive: M3 に HTTP server、daemon、RPC の実装を持ち込まずに済む。
- Negative: 人間向けの見やすい表示は別レイヤーで用意する必要がある。
- Negative: sidecar が増えるため、命名と配置の規約を厳密に守る必要がある。

## Related Requirements

- [Requirement 009](../requirements/009-okf-compatible-format.md)
- [Requirement 011](../requirements/011-graph-and-relation.md)
- [Requirement 012](../requirements/012-agent-skills.md)
- [Requirement 013](../requirements/013-api-and-cli.md)
- [Requirement 014](../requirements/014-storage-boundary.md)
- [Requirement 015](../requirements/015-query-and-filing.md)
- [Requirement 020](../requirements/020-implementation-milestones.md)
