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

## Storage Areas

- `source store`: immutable raw source。
- `wiki store`: OKF-compatible Markdown bundle。
- `metadata store`: owner、status、access policy など。
- `graph index`: Markdown link と relation から生成する derived index。
- `workflow state`: proposal、review、approval、rejection。

## Acceptance Criteria

- raw source と wiki が分離されている。
- graph index が derived であることが分かる。
- domain application との境界が明記されている。

## Related ADRs

- [ADR 008](../adr/008-separate-core-from-domain-apps.md)
- [ADR 009](../adr/009-keep-raw-sources-immutable.md)
