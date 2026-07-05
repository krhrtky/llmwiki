---
type: requirement
llmwiki:
  scope: team
  lifecycle: active
---

# Requirement 001: Vision and Problem

## Background

多くの RAG 体験では、LLM は質問のたびに raw source から関連 chunk を探し、都度合成する。この方式では、過去の合成結果、矛盾、関係、判断理由が蓄積されない。

LLMWiki は raw source と回答の間に、LLM が継続的に保守する wiki を置く。wiki は persistent and compounding artifact として、source が増えるたびに更新される。

## Problem

- 知識が raw source、会話、議事録、個人メモに散在し、Agent が再利用できない。
- 判断理由や矛盾が会話ログに埋もれ、後続実装者が同じ論点を再発見する。
- RAG だけでは、知識を育てる maintenance layer がない。

## Goals

- LLMWiki を repository-local な SoT として外化する。
- Agent が参照できる形で要求、ADR、未決論点、参照元を分離する。
- query、ingest、lint、propose の結果が wiki に蓄積される状態を目指す。

## Non-goals

- 初期段階で全文検索基盤や vector DB を必須にしない。
- raw source の全文を org scope に集約しない。
- 人間の判断を Agent に置き換えない。

## User Value

人間は、実装前に背景、前提、選択肢、懸念、未決論点を確認できる。Agent は、会話ログではなく SoT を読んで実装を継続できる。

## Acceptance Criteria

- `docs/` から LLMWiki の目的と非目的を説明できる。
- RAG ではなく wiki maintenance layer が必要な理由が明記されている。
- 関連 ADR と参照元に辿れる。

## Related ADRs

- [ADR 001](../adr/001-use-docs-as-source-of-truth.md)
- [ADR 009](../adr/009-keep-raw-sources-immutable.md)
- [ADR 010](../adr/010-use-index-and-log-for-progressive-disclosure.md)
