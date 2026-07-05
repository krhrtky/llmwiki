---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 011: Graph and Relation

## Background

LLMWiki は directory tree だけでなく、Markdown link による graph を持つ。concept、policy、procedure、decision、source が関係として辿れる必要がある。

## Problem

link がない wiki は page の集合に留まり、Agent が依存、制約、矛盾、根拠、廃止関係を判断しにくい。

## Goals

- page 間の link と relation を保守する。
- broken link、orphan page、重要概念の欠落を検出する。
- typed relation の補助 metadata は ADR 015 に従い `*.llmwiki.yaml` に保存する。
- docs、spec、ADR、implementation artifact の対応を relation で追跡できるようにする。

## Initial Relation Vocabulary

- `depends_on`
- `constrained_by`
- `implements`
- `specializes`
- `derived_from`
- `answers`
- `decided_by`
- `contradicts`
- `supersedes`
- `superseded_by`
- `related_to`
- `example_of`
- `mentions`
- `similar_to`
- `owned_by`
- `reviewed_by`
- `implemented_by`
- `verified_by`
- `enforced_by`
- `distributed_as`

`mentions` は entity または concept への言及を表す。`similar_to` は embedding、LLM、または人間が示した意味的近さを表すが、依存、制約、公式判断として扱わない。

`implemented_by` は requirement、spec、ADR を実装コード、テスト、CI gate、CLI command へつなぐ relation である。`verified_by` は claim や decision を検証する test、lint、review を表す。`enforced_by` は policy、guardrail、CI gate が行動を強制する関係を表す。`distributed_as` は skill、role skill、package などへの分配関係を表す。

relation metadata は、可能な場合に `target_kind`、`provenance`、`confidence`、`status` を持つ。`target_kind` は target の種別を示す補助情報で、`doc`、`code`、`test`、`skill`、`command`、`generated`、`external` のいずれかとする。`provenance` は `human`、`llm`、`embedding`、`parser` のいずれかを基本とし、`status` は `active`、`proposed`、`deprecated` のいずれかを基本とする。初期 deterministic lint は `type` と `target` を必須項目とし、Markdown 以外の target では `target_kind` も必須とする。追加 metadata は後方互換な拡張として扱う。

## Acceptance Criteria

- Markdown link が graph edge として扱われる。
- relation vocabulary の初期候補がある。
- typed relation の保存方式は ADR 015 に記録されている。
- 検索時に relation を辿る retrieval は、Markdown / sidecar から作る derived graph index を入力にする。

## Related ADRs

- [ADR 010](../adr/010-use-index-and-log-for-progressive-disclosure.md)
- [ADR 015](../adr/015-store-typed-relations-in-llmwiki-sidecar.md)
- [ADR 021](../adr/021-trace-docs-and-implementation-with-stable-evidence-links.md)
- [ADR 022](../adr/022-distribute-codex-skills-by-responsibility.md)
