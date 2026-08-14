# LLM Wiki ローカル規約

このディレクトリは個人用 LLM Wiki です。共通の操作手順は `llmwiki` Agent Skill に従い、
このファイルではこの Wiki に固有の知識モデルと不変条件を定義します。

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

## ローカル分類

- `sources/`: 原本ごとの要約と主張
- `concepts/`: 原本をまたぐ概念・論点
- `entities/`: 人物・組織・製品などの固有対象
- `projects/`: 期限や目的を持つ調査・活動
- `analyses/`: 将来も再利用する比較・回答・統合分析

この分類に収まらないページを作る前に、ユーザーに分類を確認します。

## 変更ログ

ingest、query、lint ごとに `wiki/log.md` の末尾へ必ず次の見出し形式で追記します。

```markdown
## [YYYY-MM-DD] operation | Title
```

`operation` は `ingest`、`query`、`lint` のいずれかです。既存のログ本文は編集、並べ替え、
削除しません。Git 管理下では CLI が HEAD の `log.md` を基準に append-only を検証します。

## ローカルな補足

- 日本語で記述します。
- PDF や画像は Agent が原本を直接読んで扱います。CLI は PDF テキスト抽出・OCR・
  ベクトル検索を実行しません。
