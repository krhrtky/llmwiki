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

## 検証済み

次の command は 2026-07-05 時点で成功済み。

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
target/debug/llmwiki lint --workspace-root .
```

`cargo test` には `tests/cli_golden.rs` の 16 tests を含む。

## 次にやるべきこと

### 1. release 前の最終監査

- clean checkout で CI と同じ command を再実行する。
- GitHub Actions 上で workflow が通ることを確認する。
- `README.md` の command examples を、実際の小さな workspace で一通り手動 smoke する。

### 2. scope model の後続判断

- `private` scope を追加するか判断する。
- 追加する場合は ADR 002、Requirement 004、format profile、CLI validation、lint/export/query tests をまとめて更新する。
- 追加しない場合は、`personal` が個人/private 相当であることを glossary または scope model に明記する。

### 3. golden tests の保守性改善

- 現状の `query` / `export` success golden は `score` や stringified decision log まで固定している。
- contract ではなく実装詳細に寄った値が増えた場合、正規化または構造化比較へ寄せる。
- ただし現時点では 8 command の success / non-success contract は固定済みであり、release blocker ではない。

### 4. release / PR 作業

- 必要なら changelog または release note を追加する。
- commit history を確認し、粒度が適切であることを確認する。
- remote に push し、CI result を確認する。

## 完了判定ルール

- slice 完了: その slice の SoT、実装、テスト、review 指摘反映、検証 command が完了した状態。
- 全体完了: M3 / M4 / M5 の command、workflow、lint が実装・検証済みで、fixture / golden / README / release / CI の品質面と docs lint の扱いまで固まっている状態。
- 今回の状態は、LLMWiki Core の初期完成条件を repository 内では満たしている。外部 CI と release 作業は次セッションで確認する。
