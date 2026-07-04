# ADR Index

ADR は技術的な意思決定を記録する。会話ログで明確に選ばれた判断は初期状態を `Accepted` とする。

| ADR | Status | Decision |
| --- | --- | --- |
| [001](./001-use-docs-as-source-of-truth.md) | Accepted | `docs/` を SoT にする |
| [002](./002-adopt-personal-team-org-scope.md) | Accepted | `personal → team → org` を採用する |
| [003](./003-use-propose-not-backport.md) | Accepted | 昇格操作を `propose` と呼ぶ |
| [004](./004-require-redaction-gate.md) | Accepted | propose に redaction gate を必須化する |
| [005](./005-use-operation-aware-access-control.md) | Accepted | 操作単位の access control を採用する |
| [006](./006-adopt-okf-compatible-markdown.md) | Accepted | OKF-compatible Markdown を採用する |
| [007](./007-extend-okf-with-llmwiki-namespace.md) | Accepted | LLMWiki 固有情報を `llmwiki` 拡張に置く |
| [008](./008-separate-core-from-domain-apps.md) | Accepted | Core と domain application を分離する |
| [009](./009-keep-raw-sources-immutable.md) | Accepted | raw source は immutable にする |
| [010](./010-use-index-and-log-for-progressive-disclosure.md) | Accepted | `index.md` と `log.md` を使う |
| [011](./011-start-with-file-and-cli-first.md) | Accepted | file + CLI first で始める |
| [012](./012-require-human-review-for-org-publish.md) | Accepted | org publish は human review 必須 |
| [013](./013-finalize-m3-cli-contract.md) | Accepted | M3 の CLI/API 契約を固定する |
