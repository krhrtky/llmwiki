# 次セッション引き継ぎ

この文書は、直近セッションで実装した範囲と、LLMWiki Core の完全完成までに残っている作業を引き継ぐためのメモである。

## 現在の状態

- `sidecar store + M5 lint target 拡張` slice は実装済み。
- typed relation の保存場所は ADR 015 で `page.llmwiki.yaml` に固定済み。
- `lint` は sidecar と frontmatter の structured claim metadata を読み、owner、reviewer、stale claim、contradiction、typed relation の一部 finding を出す。
- `ingest`、`query`、`file`、`graph`、`propose`、`redact`、`export` はまだ `hold` を返すだけで実動作しない。
- 完了判定は「slice 完了」と「LLMWiki Core 全体完了」を分けること。現状は slice 完了であり、全体完了ではない。

## 直近コミットに含めた範囲

- ADR 015: typed relation を `page.llmwiki.yaml` に保存する判断。
- Requirement 011 / Maintenance spec / open questions / ADR index の SoT 整合。
- `src/sidecar.rs` に page 隣接 sidecar reader を追加。
- `src/lint.rs` に以下の lint target を追加または拡張。
  - `docs.missing_owner`
  - `docs.missing_reviewer`
  - `docs.stale_claim`
  - `docs.contradiction`
  - `graph.unknown_relation`
  - `graph.ambiguous_relation`
  - `graph.superseded_without_target`
- frontmatter と `*.llmwiki.yaml` の structured claim metadata を `docs.stale_claim` / `docs.contradiction` の入力にする。
- sidecar schema 不正、sidecar symlink escape、optional claim value の扱いをテストで固定。

## 検証済み

- `mise exec rust -- cargo fmt --check`
- `mise exec rust -- cargo test`
- `mise exec rust -- cargo clippy --all-targets -- -D warnings`
- `mise exec rust -- cargo build`
- `target/debug/llmwiki lint --workspace-root .`

`llmwiki lint --workspace-root .` は JSON を返すが、既存 docs に frontmatter がないため `docs.missing_frontmatter` が多数出る。これは今回 slice の失敗ではないが、完全完成の判定では扱いを決める必要がある。

## 完全完成までに残っている作業

### M3 command 実装

- `ingest`
- `query`
- `file`
- `graph`
- `propose`
- `redact`
- `export`

現状は `src/main.rs` から `unsupported_command(...)` に落ち、JSON `hold` を返すだけである。各 command について入力 validation、JSON output、失敗語彙 `deny` / `hold` / `error`、workspace 外 path 拒否を実装する。

### graph command / derived index

- Markdown link から edge list を生成する。
- `*.llmwiki.yaml` の typed relation を補助 metadata として統合する。
- graph index artifact を workspace 管理下の derived artifact として出力する。
- graph command の JSON output contract を固定する。

### M5 lint target の残り

- `graph.orphan_page`
- `graph.missing_required_link`
- `docs.duplicate_concept`
- `docs.index_log_drift`

### workflow / propose / redact

- proposal draft 生成。
- `from_scope -> to_scope` の昇格 validation。
- reviewer / approver / risk_owner validation。
- redaction report、generalization notes、evidence map、diff summary。
- review / approval / rejection / hold reason の workflow sidecar 記録。
- publish 後の link plan。

### operation-aware access control

- policy object parser。
- operation ごとの `allow` / `deny` / `hold` evaluator。
- `metadata` / `summary` / `content` content level。
- decision log 出力。
- `export` / `train` など high-impact operation の制御。
- 評価順序と競合解決は未決事項なので、実装前に ADR が必要。

### ingest / query / file の知識生成フロー

- raw source immutable 境界。
- wiki candidate 生成。
- citation candidate / evidence map。
- query answer + citations + confidence。
- filing artifact 生成。
- owner / reviewer / risk_owner / access_policy_refs の要求。
- wiki 本体へ直接反映しない review candidate 保存。

### redaction scan の方式決定

- rule-based、LLM、DLP service、hybrid のどれで初期実装するかを決める。
- 初期 sensitive category の検出方式。
- sanitized draft の保存場所。
- residual risk の扱い。

### CI / release quality

- CI workflow。
- fixture based integration tests。
- CLI golden JSON tests。
- README / usage。
- install / release 手順。
- `cargo fmt`、`cargo clippy`、`cargo test`、`llmwiki lint` の gate 化。

## 次に着手する推奨順

1. `graph` command と derived index を実装する。
2. 残り M5 lint target のうち `graph.orphan_page` と `graph.missing_required_link` を `graph` model に載せる。
3. `docs.duplicate_concept` と `docs.index_log_drift` を追加する。
4. M3 の `file` command を filing artifact 生成として実装する。
5. `propose` / `redact` / workflow sidecar の実装に進む。

## 完了判定ルール

- slice 完了: その slice の SoT、実装、テスト、review 指摘反映、検証 command が完了した状態。
- 全体完了: M3 / M4 / M5 の command、workflow、lint、CI / release quality がすべて実装・検証済みで、未実装 command が `hold` のまま残っていない状態。

今後は slice 完了を全体完了として扱わない。
