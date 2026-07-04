# Requirement 003: Knowledge Types

## Background

LLMWiki は何でも保存する場所ではなく、再利用可能な知識を LLM と人間が参照・更新・検証できる形に compile する層である。

## Problem

保存対象を制限しない場合、wiki は会議ログ全文、雑談、作業メモ、監査ログ、個別問い合わせの全文で膨張し、検索性と信頼性が下がる。

## Goals

- wiki に貯める情報種別を定義する。
- wiki に貯めない情報を明示する。
- source と wiki の境界を保つ。

## Knowledge To Store

- `source_summary`: raw source の要約。
- `concept`: 再利用される概念。
- `policy`: 回答可能範囲、権限、個人情報などのルール。
- `procedure`: 手順、playbook。
- `faq`: よくある質問と回答方針。
- `decision`: ADR、意思決定。
- `guardrail`: Agent や人間が守る制約。
- `contradiction`: source 間または wiki page 間の矛盾。
- `open_question`: 未決論点。
- `glossary`: 用語、alias。

## Knowledge Not To Store

- 一時的な作業メモ。
- 個別問い合わせの全文。
- 会議ログ全文。
- Slack の雑談。
- 実装ログ。
- 監査ログ。
- 秘匿情報そのもの。
- 使うか不明な断片。

## Acceptance Criteria

- page type の初期候補が定義されている。
- raw source に残す情報と wiki に compile する情報を区別できる。
- 秘匿情報そのものを wiki に置かない方針が明記されている。

## Related ADRs

- [ADR 006](../adr/006-adopt-okf-compatible-markdown.md)
- [ADR 009](../adr/009-keep-raw-sources-immutable.md)
