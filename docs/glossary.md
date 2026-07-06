---
type: glossary
llmwiki:
  scope: team
  lifecycle: active
---

# Glossary

## LLMWiki

LLM が raw source を読み、Markdown wiki を継続的に保守する知識基盤。RAG のように毎回 raw source から再発見するのではなく、抽出・統合・矛盾検出・リンク更新を wiki に蓄積する。

## Raw Source

記事、仕様書、議事録、会話ログ、PDF、外部ドキュメントなどの元資料。原則 immutable とし、LLM は変更しない。

## Wiki

LLM が保守する Markdown の知識層。raw source の解釈、要約、概念、手順、判断、矛盾、未決論点を保持する。

## Schema

LLM に wiki の構造、命名、frontmatter、workflow、lint rule を伝える運用ルール。AGENTS.md や docs 内のルール群が該当する。

## Scope

旧互換 metadata。`personal`、`team`、`org` の値を持つ既存 page は migration input として扱う。新規の可視性境界は storage registry で表す。

## Storage Visibility Boundary

CLI が読み取り対象 root を決める物理境界。`private`、`team:<team_id>`、任意の `org` を持つ。

## private

発見、仮説、個人メモ、個人/private 相当の知識を扱う storage visibility boundary。最大 1 store とし、local path または repository を許可する。

## team

実務に耐える再利用可能な局所知識を扱う storage visibility boundary。`team_id` ごとに複数 store を許可する。

## org

組織横断で再利用される正規知識の storage visibility boundary。全情報を中央集権的に保存する場所ではなく、公式な語彙、制約、ポリシー、判断を置く層。初期は任意。

## propose

下位 store の知識を、上位 store 向けに抽象化・匿名化・根拠整理したうえで提出する操作。copy や backport ではない。

## OKF

Open Knowledge Format。Markdown と YAML frontmatter による、agent-readable な知識 bundle 形式。LLMWiki では保存形式の土台として採用する。

## Knowledge Bundle

OKF における配布単位。Markdown ファイルのディレクトリツリーであり、git repository、archive、または大きな repository 内の subdirectory として扱える。

## Concept Document

OKF における 1 つの知識単位。予約ファイル以外の `.md` ファイルであり、frontmatter に `type` を持つ。

## Operation-Aware Access Control

単なる visibility ではなく、`read`、`search`、`retrieve`、`query`、`propose`、`export`、`train` などの操作単位で参照可否を決める制御。

## Redaction Gate

秘匿情報が上位 store へ混入しないように、propose の前後で検査・匿名化・抽象化・拒否を行う必須ゲート。

## Harness Engineering

Agent が信頼できる仕事をするために、SoT、検証、ツール、ガードレール、観測可能性を repository 内に整備する考え方。
