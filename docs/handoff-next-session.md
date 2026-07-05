# 次セッション引き継ぎ

この文書は、LLMWiki Core の完全完成までに残っている作業を次セッションの実行者へ引き継ぐためのメモである。

## 現在の状態

- `sidecar store + M5 lint target 拡張` slice は実装・検証済み。
- typed relation の保存場所は ADR 015 で `page.llmwiki.yaml` に固定済み。
- `lint` は sidecar と frontmatter の structured claim metadata を読み、owner、reviewer、stale claim、contradiction、typed relation、orphan、required link、duplicate concept、index/log drift の finding を出す。
- `ingest`、`query`、`file`、`graph`、`propose`、`redact`、`export` は deterministic な最小実装が入っており、JSON contract を返す。
- `ingest` と `query` は実装・検証・レビュー済み。
- 完了判定は「slice 完了」と「LLMWiki Core 全体完了」を分けること。現状は slice 完了であり、全体完了ではない。

## 実装済み範囲

- ADR 016: access policy 評価順序を `deny > hold > allow`、no match は `hold` に固定。
- ADR 017: 初期 redaction scan を rule-based deterministic に固定。
- ADR 018: semantic maintenance detection と source 更新ベース stale detection を初期完成の必須範囲から外す。
- `src/access.rs`: operation-aware access evaluator。
- `src/graph.rs`: Markdown link と `*.llmwiki.yaml` relation から graph index を生成。
- `src/file.rs`: filing artifact を `.llmwiki/filings/` に生成。
- `src/propose.rs`: redaction report を前提に proposal draft を生成。
- `src/redact.rs`: rule-based redaction report と sanitized draft を生成。
- `src/export.rs`: access evaluator を通した export artifact を生成。
- `src/ingest.rs`: raw source から deterministic な wiki candidate を生成。raw source は変更しない。
- `src/query.rs`: deterministic lexical query。query は read-only で filing は自動実行しない。
- `src/lint.rs`: M5 lint target を deterministic に検査。

## 検証済み

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo build`
- `target/debug/llmwiki lint --workspace-root .`
- CLI smoke: `ingest` success / invalid scope hold。
- CLI smoke: 日本語 `query` success / invalid scope hold / `.llmwiki` 書き込みなし。

`llmwiki lint --workspace-root .` は JSON を返すが、既存 docs に frontmatter がないため `docs.missing_frontmatter` が多数出る。これは今回 slice の失敗ではないが、完全完成の判定では扱いを決める必要がある。

## 完全完成までに残っている作業

### 1. docs lint gate 方針

- 既存 docs に frontmatter を補完して `docs.missing_frontmatter` を解消するか、reserved / SoT docs の例外 rule を lint に追加するか決める。
- release gate で許容する finding と失敗扱いにする finding を明文化する。
- 決定が lint rule に影響する場合は、必要に応じて ADR または maintenance spec に記録する。

### 2. fixture / golden tests

- `tests/fixtures/` 相当の小さな workspace を作り、主要 command の integration test を追加する。
- CLI JSON の golden test を command ごとに追加する。
- timestamp や path など実行環境で変わる値は正規化して比較する。
- 最低対象 command: `lint`, `graph`, `ingest`, `query`, `file`, `redact`, `propose`, `export`。

### 3. README / usage / release

- README または usage doc に、workspace layout、policy file 例、各 command の最小実行例を書く。
- install / release 手順を追加する。
- `cargo fmt`、`cargo clippy`、`cargo test`、`llmwiki lint` を CI gate に載せる。

## 次に着手する推奨順

1. `docs.missing_frontmatter` の扱いを決める。実装分岐があるため、ここを先に固める。
2. 決定に従って frontmatter 補完または lint exception rule を実装する。
3. fixture based integration tests と CLI golden JSON tests を追加する。
4. README / usage と install / release 手順を整備する。
5. CI workflow を追加し、`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`llmwiki lint` を gate 化する。

## 次セッション開始時の確認コマンド

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
target/debug/llmwiki lint --workspace-root .
```

`llmwiki lint --workspace-root .` は現時点で `docs.missing_frontmatter` を返す想定である。次セッションではこの finding を release gate で許容するか、docs または lint rule を変更して解消するかを最初に決める。

## 完了判定ルール

- slice 完了: その slice の SoT、実装、テスト、review 指摘反映、検証 command が完了した状態。
- 全体完了: M3 / M4 / M5 の command、workflow、lint が実装・検証済みで、fixture / golden / README / release / CI の品質面と docs lint の扱いまで固まっている状態。

今後は slice 完了を全体完了として扱わない。
