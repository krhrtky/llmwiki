# LLM Wiki 運用規約

このディレクトリは個人用 LLM Wiki です。LLM Agent は、以下の規約を守って
`raw/`、`wiki/`、`.llmwiki/` を操作してください。

## 境界

- `raw/` は不変の原本です。`llmwiki source add` 以外の方法で作成、変更、移動、
  削除しません。原本の内容を整形・OCR・抽出して置き換えることもしません。
- `.llmwiki/manifest.json` は CLI の管理対象です。手で編集しません。
- `wiki/` は Agent が管理する永続的な知識層です。通常の Markdown リンクではなく、
  `wiki/` を基準とした拡張子なし Obsidian リンクを使います。例:
  `[[sources/market-report]]`。raw への添付は vault root から `![[raw/report.pdf]]` の
  ように書きます。
- `wiki/index.md` と `wiki/log.md` は通常の knowledge page でも orphan 判定対象でもありません。

## ページの共通形式

`wiki/` の content page には YAML frontmatter を置き、少なくとも次を設定します。

```yaml
---
title: ページタイトル
type: source # または concept, entity, project, analysis
created: "2026-08-14"
updated: "2026-08-14"
---
```

- ファイル名は UTF-8 の kebab-case にします。
- knowledge page（`source` 以外）には、非空の `sources` を追加し、根拠となる
  source page を Obsidian リンクで列挙します。単なるパスではなく、たとえば
  `sources: ["[[sources/market-report]]"]` の形式にします。
- source page には `source_file` と `source_sha256` を追加します。前者は raw 内の
  vault-root 相対パス、後者は `llmwiki source add` が manifest に登録した SHA-256 です。

## Ingest

ユーザーが原本の取り込みを依頼したら、事前承認を待たずに以下を実行します。

1. `llmwiki source add TARGET FILE...` で原本を登録する。すでに raw に登録済みなら、
   manifest と SHA-256 を確認し、原本を変更しない。
2. 原本と `wiki/index.md` を読み、`wiki/sources/` に source page を作成または更新する。
   source page は原本リンク（例: `![[raw/report.pdf]]` または `![[raw/notes.txt]]`）、
   SHA-256、`## Summary` の要約、`## Claims` の主要主張を必ず含める。
3. 既存の concept/entity/project page を再利用・更新し、原本をまたぐ主張を統合する。
   知識ページには source page への根拠リンクを必ず残す。
4. 主張が矛盾する場合、どちらかを消して解決したことにしない。両方の source page と
   主張をリンクし、ページ内の `## Contradictions` 節に不確実性または条件の違いを
   記録する。
5. 新しい content page を `wiki/index.md` からリンク可能にし、`wiki/log.md` に追記する。
6. 完了時には、変更したページの一覧と、追加・更新・矛盾統合という主要な統合内容を
   ユーザーへ提示する。

## Query

問い合わせでは、最初に `wiki/index.md` を読んで関連ページをたどります。根拠を
source page と raw 原本まで追跡し、回答では不確実性と矛盾を区別します。将来も再利用
できる新規分析だけを Agent の判断で `wiki/analyses/` に保存し、そのページにも非空の
`sources` を設定します。保存した場合は log に追記します。

## Semantic lint

Agent は意味的な品質を確認します。矛盾、陳腐化、ページ化されていない重要概念、根拠や
情報の不足を検査し、必要なら Wiki を更新またはユーザーに確認します。source page の
Summary/Claims と knowledge page の Contradictions の内容が十分かどうかも Agent の責務です。
CLI の
`llmwiki check` は frontmatter、リンク、index、orphan、raw hash、log などの構造検査を
担い、意味の正しさは判定しません。

## 変更ログ

ingest、query、lint ごとに `wiki/log.md` の末尾へ必ず次の見出し形式で追記します。

```markdown
## [YYYY-MM-DD] operation | Title
```

`operation` は `ingest`、`query`、`lint` のいずれかです。既存のログ本文は編集、並べ替え、
削除しません。Git 管理下では CLI が HEAD の `log.md` を基準に append-only を検証します。

## 最終確認

Wiki を編集した後は `llmwiki check TARGET` を実行し、構造診断を解消します。検索には
`llmwiki search TARGET QUERY` を使えます。PDF や画像は Agent が原本を直接読んで扱い、
CLI は PDF テキスト抽出・OCR・ベクトル検索を実行しません。
