# LLM Wiki Personal MVP

不変のローカル原本 (`raw/`)、Agent が統合する永続 Wiki (`wiki/`)、その運用規約
(`AGENTS.md`) を作るための Rust CLI です。個人用（personal）プロファイルだけを提供します。

## Rust toolchain とビルド

Rustup で stable toolchain を準備します。リポジトリの `rust-toolchain.toml` が
`rustfmt` と `clippy` も指定しているため、初回の Cargo 実行時に必要な component が
導入されます。

```console
rustup toolchain install stable
cargo build --release
./target/release/llmwiki --help
```

## Agent Skill

複数のリポジトリで同じ Wiki 運用を使うための `llmwiki` Agent Skill を
[`skills/llmwiki`](skills/llmwiki) に同梱しています。ローカル clone から Codex に
インストールするには、CLI をインストールした後に次を実行します。

```console
llmwiki skill install
```

インストール後は、他リポジトリで `$llmwiki` を指定して Wiki の初期化、取り込み、照会、
semantic lint を依頼できます。Skill は共通手順を担い、各 Wiki に生成される `AGENTS.md` は
そのリポジトリ固有の分類・データ方針を担います。既存の Skill はバージョンが古い場合にだけ
自動更新し、同一または新しいバージョンは変更しません。

ローカル環境へコマンドをインストールして以後 `llmwiki` として使う場合は、リポジトリの
ルートで次を実行します。

```console
cargo install --path .
llmwiki --help
```

以降の例では、`llmwiki` が PATH にあるものとします。`cargo install` を使わない場合は、
各 `llmwiki` を `./target/release/llmwiki` に置き換えてください。

開発時の品質確認は Cargo だけで実行できます。

```console
cargo test
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release
```

## 最短の再現手順

### 1. Wiki を初期化する

`TARGET` は空のディレクトリか、まだ存在しないパスを指定します。既存ファイルがある
ディレクトリを指定した場合、`init` は上書きせず失敗します。

```console
llmwiki init ~/knowledge/personal-wiki
llmwiki check ~/knowledge/personal-wiki
```

初期化すると、次の構造が作られます。

```text
personal-wiki/
├── AGENTS.md
├── raw/
├── wiki/
│   ├── index.md
│   ├── log.md
│   ├── sources/
│   ├── concepts/
│   ├── entities/
│   ├── projects/
│   └── analyses/
└── .llmwiki/
    └── manifest.json
```

`AGENTS.md` はこの Wiki 固有の規約です。`raw/` と `.llmwiki/manifest.json` を手で編集
しないこと、ページ分類、言語などを定義します。複数のリポジトリで共通の取り込み・照会・
semantic lint 手順を使う場合は、`llmwiki` Agent Skill をインストールして利用します。

### 2. 原本を登録する

ローカルの `.md`、`.txt`、`.pdf`、`.png`、`.jpg`、`.jpeg`、`.webp`、`.gif`、`.svg` を
登録できます。CLI はファイルを `raw/` にコピーし、SHA-256 を manifest に登録します。

```console
llmwiki source add ~/knowledge/personal-wiki ./notes/remote-work.md ./figures/survey.png
llmwiki check ~/knowledge/personal-wiki
```

同一内容の再登録は安全に繰り返せます。同名で内容が異なる原本は、既存の原本を上書きせず
エラーになります。登録後のハッシュは `.llmwiki/manifest.json` で確認できます。内部形式は
公開 API ではありませんが、登録内容は次のように保存されます。

```json
{
  "sources": {
    "raw/remote-work.md": { "sha256": "..." }
  }
}
```

### 3. Agent に ingest を依頼する

`llmwiki` Agent Skill と Wiki 直下の `AGENTS.md` に従うよう、次のように依頼します。

```text
`$llmwiki` を使い、AGENTS.md に従って raw/remote-work.md を ingest してください。
既存の index を先に確認し、source page と必要な knowledge page を更新してください。
完了時は変更ページと統合内容を報告してください。
```

Agent は事前承認を待たずに Wiki を更新しますが、`raw/` は `llmwiki source add` 以外で
変更しません。source page の最小例は次のとおりです。`source_sha256` は manifest の実値を
使います。

```markdown
---
title: Remote Work Survey
type: source
created: "2026-08-14"
updated: "2026-08-14"
source_file: raw/remote-work.md
source_sha256: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
---

# Remote Work Survey

![[raw/remote-work.md]]

## Summary

この調査は、週 3 日の在宅勤務が通勤時間を削減したと報告する。

## Claims

- 通勤時間の削減は回答者の満足度と同時に報告された。
```

knowledge page は `source` 以外の type を使い、根拠 source page の Obsidian link を
非空の `sources` に指定します。`sources` に単なるパス文字列を書かないでください。

```markdown
---
title: Remote Work Effects
type: concept
created: "2026-08-14"
updated: "2026-08-14"
sources:
  - "[[sources/remote-work-survey]]"
---

# Remote Work Effects

在宅勤務の影響は、対象者と評価指標によって異なる。

## Evidence

- [[sources/remote-work-survey]]

## Contradictions

別の source が反対の主張をする場合は、その source page と主張の両方をここに追加する。
どちらかの主張を削除して矛盾を隠さない。
```

Agent は新しい content page を `wiki/index.md` から到達可能にします。index 内の Wiki link
は `wiki/` を基準とし、拡張子を省略します（例: `[[sources/remote-work-survey]]`）。

### 4. 検索する

検索は Wiki の Markdown と raw 内の Markdown/テキストに対する、大文字小文字を区別しない
字句検索です。日本語もそのまま検索できます。PDF・画像などのバイナリは検索対象外です。

```console
llmwiki search ~/knowledge/personal-wiki "在宅勤務"
llmwiki search ~/knowledge/personal-wiki "remote work"
llmwiki search ~/knowledge/personal-wiki "no matching phrase"
```

各一致はタイトル、種別、パス、行番号、該当スニペットとともに表示されます。埋め込み検索は
行いません。

### 5. query と lint を Agent に依頼する

query では必ず index を最初に読み、関連ページと source page、さらに raw 原本まで根拠を
たどります。再利用価値がある新規分析だけを Agent の判断で `wiki/analyses/` に保存します。

```text
`$llmwiki` を使い、AGENTS.md に従って「在宅勤務は満足度を改善するか」を query してください。
index と根拠を確認し、再利用価値がある新規分析だけを analyses/ に保存してください。
```

semantic lint は CLI ではなく Agent が担います。矛盾、陳腐化、未ページ化の重要概念、根拠
不足を検査します。source page の Summary/Claims と knowledge page の Contradictions が
内容として十分かどうかも Agent が判断します。

```text
`$llmwiki` を使い、AGENTS.md に従って semantic lint を実行し、矛盾・陳腐化・未ページ化概念・情報不足を確認してください。
必要な更新と、判断できない点を報告してください。
```

ingest、query、lint のたびに Agent は `wiki/log.md` の末尾へ、次の H2 見出しで追記します。

```markdown
## [2026-08-14] ingest | Remote Work Survey
```

`operation` は `ingest`、`query`、`lint` のいずれか、Title は空にしません。既存のログは
削除・編集・並べ替えません。

### 6. 構造を検証する

```console
llmwiki check ~/knowledge/personal-wiki
llmwiki check ~/knowledge/personal-wiki --format json
```

`check` は raw の欠損・ハッシュ変更、manifest、frontmatter、Obsidian link、index 到達性、
orphan、log 見出しを検査します。`[[page]]`、`[[page|alias]]`、`[[page#heading]]`、
`![[raw/file.pdf]]` を解決します。成功時の終了コードは `0`、検証問題は `1`、利用方法または
実行環境のエラーは `2` です。`--format json` は次の機械可読な診断を返します。

```json
{
  "diagnostics": [
    {
      "severity": "error",
      "code": "broken-link",
      "path": "wiki/concepts/example.md",
      "message": "...",
      "line": 12,
      "candidates": []
    }
  ],
  "summary": { "errors": 1, "warnings": 0 }
}
```

`line` と `candidates` は該当する診断にだけ含まれます。診断がない場合も `diagnostics` と
`summary` は出力されます。

Git 管理下で HEAD に `wiki/log.md` がある場合、`check` は現行ログがその完全な接頭辞である
ことも確認します。初回コミット前はこの履歴検査を行えないため、形式だけを検査して警告します。

## MVP の対象外

この MVP は次を実装しません。

- team profile、team 向け分類、owner/status 規約
- LLM API の呼び出し
- URL/Web の取得
- OCR と PDF テキスト抽出
- ベクトル検索・埋め込み検索

PDF と画像は不変の原本として登録でき、Agent が直接読んで source page を作ります。
