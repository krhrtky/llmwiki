# LLMWiki

LLMWiki は file-first の知識基盤 CLI です。`llmwiki` は JSON を正式出力として返し、workspace root 直下の Markdown bundle を対象に動きます。

## Workspace Layout

```text
.
├── AGENTS.md
├── Cargo.toml
├── src/
├── docs/
├── .llmwiki/
│   ├── filings/
│   ├── ingests/
│   ├── proposals/
│   ├── redactions/
│   └── exports/
└── pages-or-docs/
```

- workspace root は `index.md`、`AGENTS.md`、`docs/index.md` のいずれかを持つ bundle root として扱われます。
- `src/` は CLI と内部 command 実装です。
- `docs/` は要求、ADR、仕様の SoT です。
- `.llmwiki/` は生成物の置き場です。

## Access Policy 最小例

`query` と `export` は `--access-policy` で YAML file を受け取ります。最小例は次の形です。

```yaml
policy:
  policy_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  decision: allow
  reason: allow query
```

- `query` では `operation: query` を使います。
- `export` では同じ形で `operation: export` に変えます。
- `scope`、`content_level`、`subject.kind`、`subject.id` は CLI の指定と揃えます。

## Command Examples

`format` は現状 `json` のみです。最小実行例は次のとおりです。

```bash
llmwiki ingest --workspace-root . --scope team docs/source.md
llmwiki query --workspace-root . --question "What changed?" --scope team --content-level content --subject-kind user --subject-id alice --access-policy policy.yaml
llmwiki file --workspace-root . --candidate drafts/query.md --scope team --owner alice --reviewer bob --confidence high --citation "docs/source.md#note" --access-policy-ref policy-1
llmwiki graph --workspace-root .
llmwiki redact --workspace-root . --target-scope team docs/source.md
llmwiki propose --workspace-root . --from-scope personal --to-scope team --reviewer bob --approver carol --redaction-report .llmwiki/redactions/source.report.json drafts/page.md
llmwiki export --workspace-root . --scope team --content-level content --subject-kind user --subject-id alice --access-policy policy.yaml
llmwiki lint --workspace-root .
```

- `ingest` は raw source から candidate を作ります。
- `query` は read-only で、filing は自動実行しません。
- `file` は `--candidate`、`--scope`、`--owner`、`--confidence`、`--citation`、`--access-policy-ref` を最低限使います。
- `propose` は `--from-scope`、`--to-scope`、`--reviewer`、`--approver`、`--redaction-report` を使います。
- `redact` は `--target-scope` を省略すると `hold` に倒れることがあります。
- `graph` と `lint` は `paths` を省略すると workspace 全体を走査します。
- `export` は `--scope` か page 側の scope 情報と `--access-policy` を使って出力を制御します。

## Install

ローカル利用は次で十分です。

```bash
cargo install --path .
```

開発中は次でも動きます。

```bash
cargo build
target/debug/llmwiki --help
```

## Release

release 前は次の gate を通します。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
target/debug/llmwiki lint --workspace-root .
```

release artifact を作る場合は次を使います。

```bash
cargo build --release
```

`target/release/llmwiki` を配布対象にします。

## CI / Gate

CI は stable Rust で次を gate にします。

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build`
- `target/debug/llmwiki lint --workspace-root .`
