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

## Retrieval Scope 最小例

`query` と `related` は `--retrieval-scope` で YAML file を受け取ります。最小例は次の形です。

```yaml
retrieval_scope:
  rule_id: team-query
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
  reason: include team content for query
```

- `query` では `operation: query` を使います。
- `related` では同じ形で `operation: answer_suggestion` などに変えます。
- `scope`、`content_level`、`subject.kind`、`subject.id` は CLI の指定と揃えます。

`export` は同じ field を持つ `export_scope` YAML を `--export-scope` で受け取ります。

## Command Examples

`format` は現状 `json` のみです。最小実行例は次のとおりです。

```bash
llmwiki ingest --workspace-root . --scope team docs/source.md
llmwiki query --workspace-root . --question "What changed?" --scope team --content-level content --subject-kind user --subject-id alice --retrieval-scope retrieval-scope.yaml
llmwiki related --workspace-root . --scope team --operation answer_suggestion --content-level content --subject-kind user --subject-id alice --retrieval-scope retrieval-scope.yaml docs/procedure.md
llmwiki file --workspace-root . --candidate drafts/query.md --scope team --owner alice --reviewer bob --confidence high --citation "docs/source.md#note"
llmwiki graph --workspace-root .
llmwiki redact --workspace-root . --target-scope team docs/source.md
llmwiki propose --workspace-root . --from-scope personal --to-scope team --reviewer bob --approver carol --redaction-report .llmwiki/redactions/source.report.json drafts/page.md
llmwiki export --workspace-root . --scope team --content-level content --subject-kind user --subject-id alice --export-scope export-scope.yaml
llmwiki lint --workspace-root .
```

- `ingest` は raw source から candidate を作ります。
- `query` は read-only で、filing は自動実行しません。
- `related` は seed page から relation graph を辿る read-only operation です。
- `file` は `--candidate`、`--scope`、`--owner`、`--confidence`、`--citation` を最低限使います。
- `propose` は `--from-scope`、`--to-scope`、`--reviewer`、`--approver`、`--redaction-report` を使います。
- `redact` は `--target-scope` を省略すると `hold` に倒れることがあります。
- `graph` と `lint` は `paths` を省略すると workspace 全体を走査します。
- `export` は `--scope` か page 側の scope 情報と `--export-scope` を使って出力対象を制御します。

### Demo Helper

Codex のローカル session 履歴を LLMWiki demo 用の personal candidate に変換する場合は次を使います。raw session log は repository にコピーせず、要約 Markdown と `.llmwiki/` 配下の manifest だけを生成します。

```bash
llmwiki codex-session import --workspace-root . --sessions-root ~/.codex/sessions --repo-root . --limit 1
```

生成された `docs/personal/codex-sessions/*.md` は通常の wiki page と同じく `redact` と `propose` に渡せます。

### Distribution Helper

```bash
llmwiki skill install --workspace-root . --codex-home ~/.codex
```

- `skill install` は `skills/*/SKILL.md` を Codex skill directory へ配置する配布補助です。
- インストール対象は入口 skill `llmwiki` と用途別 skill `llmwiki-answer-query`、`llmwiki-ingest-source`、`llmwiki-file-knowledge`、`llmwiki-promote-knowledge`、`llmwiki-maintain`、`llmwiki-export` です。

## Install

Repository を clone し、release build した binary を `PATH` 上の directory に配置します。

```bash
git clone https://github.com/krhrtky/llmwiki.git
cd llmwiki
cargo build --release
mkdir -p ~/.local/bin
cp target/release/llmwiki ~/.local/bin/llmwiki
export PATH="$HOME/.local/bin:$PATH"
llmwiki --help
```

zsh で `PATH` 追加を永続化する場合は次を実行します。

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
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
