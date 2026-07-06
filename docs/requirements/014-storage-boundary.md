---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 014: Storage Boundary

## Background

LLMWiki は source、wiki、metadata、graph、workflow state を扱う。これらは更新頻度、所有者、監査要件が異なる。

## Problem

すべてを 1 つの store に押し込むと、immutable source と LLM-maintained wiki、derived graph、workflow state の責務が混ざる。

## Goals

- storage boundary を明確にする。
- 初期は file-first とし、derived index は再生成可能にする。
- domain application の業務データと LLMWiki Core の知識データを分ける。
- visibility boundary は root `llmwiki.yaml` の storage registry で解決する。

## Storage Areas

- `source store`: immutable raw source。
- `wiki store`: OKF-compatible Markdown bundle。
- `metadata store`: owner、status、citation metadata、confidence など。
- `graph index`: Markdown link と relation から生成する derived index。
- `workflow state`: proposal、review、approval、rejection。

## Visibility Store Registry

- `private`: 0 or 1 store。local path または repository を許可する。
- `team`: 0 件以上。各 entry は `team_id`、`repository`、`path` を持つ。
- `org`: 0 or 1 store。設定する場合は `repository`、`path` を持つ。

`team_id`、repository identity、canonical root は registry 内で重複してはならない。`org` 未設定時、`team -> org` の propose は実行不可とする。

## Acceptance Criteria

- raw source と wiki が分離されている。
- graph index が derived であることが分かる。
- domain application との境界が明記されている。
- CLI が `--store` から canonical root を解決し、選択 store の外側を読まないことが明記されている。

## Related ADRs

- [ADR 008](../adr/008-separate-core-from-domain-apps.md)
- [ADR 009](../adr/009-keep-raw-sources-immutable.md)
- [ADR 023](../adr/023-use-storage-registry-for-visibility-boundaries.md)
