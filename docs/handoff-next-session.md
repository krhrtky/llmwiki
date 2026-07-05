---
type: handoff
llmwiki:
  scope: team
  lifecycle: active
---

# 次セッション引き継ぎ

この文書は、次セッションの実行者が LLMWiki Core の現状態を確認し、次に着手する作業を迷わないためのメモである。

## 現在の状態

- `docs.missing_frontmatter` の扱いは、既存 SoT docs へ frontmatter を補完する方針で確定した。
- 既存 SoT docs は `llmwiki.scope: team`、`llmwiki.lifecycle: active` として初期登録済み。
- `private` は現行 scope には含めず、個人相当の知識は `personal` として扱う方針を `docs/specs/format-profile.md` に記録済み。
- `docs/adr/log.md` と `docs/requirements/log.md` を追加し、`docs.index_log_drift` を解消済み。
- `tests/cli_golden.rs` に `lint`、`graph`、`ingest`、`query`、`file`、`redact`、`propose`、`export` の CLI golden tests を追加済み。
- 各 command について success golden と non-success golden を 1 本以上持つ。
- `README.md` に workspace layout、access policy 例、各 command の最小実行例、install / release 手順、gate を記録済み。
- `.github/workflows/ci.yml` に CI gate を追加済み。
- `target/debug/llmwiki lint --workspace-root .` は findings 0 を返す。

## 実装済み範囲

- deterministic CLI/API:
  - `ingest`
  - `query`
  - `file`
  - `graph`
  - `propose`
  - `redact`
  - `export`
  - `lint`
- M5 lint:
  - frontmatter / sidecar の structured metadata 検査。
  - owner、reviewer、stale claim、contradiction、typed relation、orphan、required link、duplicate concept、index/log drift の finding。
- docs lint gate:
  - 既存 SoT docs の required frontmatter 補完。
  - ADR / requirements の `log.md` 追加。
  - `docs/specs/format-profile.md` への初期補完方針追記。
- quality gate:
  - CLI golden tests。
  - README / usage / release 手順。
  - GitHub Actions CI。
- `private` scope 判断:
  - `private` は独立 scope として追加しない。
  - 個人/private 相当の知識は `personal` と operation-aware access control で扱う。
  - 判断は ADR 019、Requirement 004、glossary、format profile に反映済み。
- related retrieval 外化:
  - grep ではなく search、graph traversal、access filter、rerank/explain に分ける方針を ADR 020 に記録済み。
  - Post-M5 の実装仕様は `docs/specs/retrieval.md` に外化済み。
  - Post-M5 の最初の固定 CLI/API は `llmwiki related` とし、`llmwiki retrieve` は後続に送る判断を ADR 020 と retrieval 仕様に記録済み。
  - relation vocabulary に `mentions` と `similar_to` を追加済み。
- related retrieval 実装:
  - `llmwiki related <seed>` を追加済み。
  - Markdown link と `*.llmwiki.yaml` relations から作る file-first derived graph index を入力にする。
  - access check は seed、edge、neighbor、section body の各段階で行う。
  - output は `related_result` JSON envelope とし、relation path と access decision を含める。
  - DB、vector DB、OpenSearch、Neo4j、GraphRAG は必須にしていない。
- golden tests 保守性改善:
  - `query` success golden は `score` の exact 値を固定せず、decision log は JSON 文字列の exact 比較ではなく構造検証へ変更済み。
  - `export` success golden は decision log を構造検証へ変更済み。
  - `related` success / hold golden tests を追加済み。

## 検証済み

次の command は 2026-07-05 時点で成功済み。

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
target/debug/llmwiki lint --workspace-root .
```

`cargo test` には `tests/cli_golden.rs` の 18 tests を含む。

## 次にやるべきこと

### 1. release 前の最終監査

- clean checkout で CI と同じ command を再実行する。
- GitHub Actions 上で workflow が通ることを確認する。
- `README.md` の command examples を、実際の小さな workspace で一通り手動 smoke する。

### 2. retrieval の後続実装

- `llmwiki retrieve` の lexical seed selection を設計する。
- section seed、section chunk、BM25、embedding index の最小 schema を固定する。
- relation proposal の human approval workflow を `file`、`propose`、または新 command のどれに接続するか決める。

### 3. retrieval の後続 ADR/設計

- section chunk、BM25、embedding index、relation proposal workflow は ADR 020 の Open Questions に残っている。
- PostgreSQL + pgvector、OpenSearch / Elasticsearch、Neo4j、GraphRAG は derived index adapter 候補であり、SoT にはしない。

### 4. release / PR 作業

- 必要なら changelog または release note を追加する。
- commit history を確認し、粒度が適切であることを確認する。
- remote に push し、CI result を確認する。

## 完了判定ルール

- slice 完了: その slice の SoT、実装、テスト、review 指摘反映、検証 command が完了した状態。
- 全体完了: M3 / M4 / M5 の command、workflow、lint が実装・検証済みで、fixture / golden / README / release / CI の品質面と docs lint の扱いまで固まっている状態。
- 今回の状態は、LLMWiki Core の初期完成条件を repository 内では満たしている。外部 CI と release 作業は次セッションで確認する。
