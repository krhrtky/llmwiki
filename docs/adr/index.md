# ADR Index

ADR は技術的な意思決定を記録する。会話ログで明確に選ばれた判断は初期状態を `Accepted` とする。

| ADR | Status | Decision |
| --- | --- | --- |
| [001](./001-use-docs-as-source-of-truth.md) | Accepted | `docs/` を SoT にする |
| [002](./002-adopt-personal-team-org-scope.md) | Superseded by [023](./023-use-storage-registry-for-visibility-boundaries.md) | `personal → team → org` を採用する |
| [003](./003-use-propose-not-backport.md) | Accepted | 昇格操作を `propose` と呼ぶ |
| [004](./004-require-redaction-gate.md) | Accepted | propose に redaction gate を必須化する |
| [005](./005-use-operation-aware-access-control.md) | Accepted | 操作単位の access control と scope rule 最小契約を採用する |
| [006](./006-adopt-okf-compatible-markdown.md) | Accepted | OKF-compatible Markdown を採用する |
| [007](./007-extend-okf-with-llmwiki-namespace.md) | Accepted | LLMWiki 固有情報を `llmwiki` 拡張に置く |
| [008](./008-separate-core-from-domain-apps.md) | Accepted | Core と domain application を分離する |
| [009](./009-keep-raw-sources-immutable.md) | Accepted | raw source は immutable にする |
| [010](./010-use-index-and-log-for-progressive-disclosure.md) | Accepted | `index.md` を使い、`log.md` は必要に応じて使う |
| [011](./011-start-with-file-and-cli-first.md) | Accepted | file + CLI first で始める |
| [012](./012-require-human-review-for-org-publish.md) | Accepted | org publish は human review 必須 |
| [013](./013-finalize-m3-cli-contract.md) | Accepted | M3 の CLI/API 契約を固定する |
| [014](./014-finalize-m5-maintenance-contract.md) | Accepted | M5 の maintenance contract を固定する |
| [015](./015-store-typed-relations-in-llmwiki-sidecar.md) | Accepted | typed relation を `page.llmwiki.yaml` に置く |
| [016](./016-finalize-access-policy-evaluation.md) | Accepted | scope rule の評価順序を `exclude > hold > include` に固定する |
| [017](./017-use-rule-based-redaction-scan-initially.md) | Accepted | 初期 redaction scan は rule-based deterministic にする |
| [018](./018-defer-semantic-maintenance-detection.md) | Accepted | semantic maintenance detection は初期完成後に送る |
| [019](./019-do-not-add-private-scope.md) | Superseded by [023](./023-use-storage-registry-for-visibility-boundaries.md) | `private` scope は追加せず、個人/private 相当は `personal` と scope rule で扱う |
| [020](./020-use-hybrid-search-and-graph-traversal-for-related-retrieval.md) | Accepted | related retrieval は hybrid search と graph traversal を責務分離して設計する |
| [021](./021-trace-docs-and-implementation-with-stable-evidence-links.md) | Accepted | docs と implementation を stable な証跡で相互参照する |
| [022](./022-distribute-codex-skills-by-responsibility.md) | Accepted | LLMWiki を Codex Skill として配布する |
| [023](./023-use-storage-registry-for-visibility-boundaries.md) | Accepted | storage registry で visibility boundary を物理分離する |
| [024](./024-delegate-store-edit-authorization-to-repository-controls.md) | Accepted | store の直接編集権限は repository controls に委譲する |
| [025](./025-rename-access-policy-vocabulary-to-scope-rules.md) | Accepted | CLI/API の対象範囲制御語彙を scope rule へ置き換える |
