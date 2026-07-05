# 未決事項

このファイルは未決事項を集約する。実装者は未決事項を推測で埋めず、実装判断が分岐する場合はユーザー判断または追加 ADR を作成する。

## 後続 ADR 候補

- policy lint の詳細 rule。
- redaction rule set の具体 pattern と severity。
- LLM または DLP service を redaction scan に追加する条件。
- source 更新に基づく stale claim 検出の metadata contract。
- 本文意味比較による contradiction / stale 検出の実装方式。

## 解決済み

- M3 では CLI を単一 binary `llmwiki` とし、内部関数 API を正本にする。詳細は [ADR 013](./adr/013-finalize-m3-cli-contract.md)。
- M3 では metadata store と workflow state store に page 隣接 sidecar (`page.llmwiki.yaml`, `page.workflow.yaml`) を採用する。詳細は [ADR 013](./adr/013-finalize-m3-cli-contract.md)。
- M3 では CLI/API output の正式 transport を JSON のみに固定し、Markdown report は派生表示として扱う。詳細は [ADR 013](./adr/013-finalize-m3-cli-contract.md)。
- typed relation の補助 metadata は `page.llmwiki.yaml` に保存する。Markdown link は graph edge の正本であり、typed relation は補助 metadata として扱う。詳細は [ADR 015](./adr/015-store-typed-relations-in-llmwiki-sidecar.md)。
- access policy の評価順序は `deny > hold > allow` とし、一致 policy がない場合は `hold` とする。specificity は同一 decision 内の説明対象選択に使う。詳細は [ADR 016](./adr/016-finalize-access-policy-evaluation.md)。
- 初期 redaction scan は rule-based deterministic とし、redaction report と sanitized draft は `.llmwiki/redactions/` に保存する。詳細は [ADR 017](./adr/017-use-rule-based-redaction-scan-initially.md)。
- 本文意味比較による contradiction / stale 検出と source 更新に基づく stale claim 検出は初期完成の必須範囲に含めない。詳細は [ADR 018](./adr/018-defer-semantic-maintenance-detection.md)。

## ユーザー判断待ち

- 初期ユースケースを個人 wiki、チーム wiki、問い合わせ管理、またはハーネスエンジニアリング基盤のどれに寄せるか。
- 秘匿情報の最終分類を社内ポリシーと合わせるか、LLMWiki 独自の初期分類で始めるか。

## 解決済み（M4）

- `hold` は lifecycle state に昇格させず、policy/review decision として `*.workflow.yaml` に記録する。decision log は監査用の派生記録とする。
- operation-aware access control は policy object と decision log の最小項目まで M4 で固定し、評価順序と競合解決は ADR 016 で固定した。詳細は [ADR 016](./adr/016-finalize-access-policy-evaluation.md)。
- `org` publish の承認者は human role とし、基本は `domain_owner`、sensitive category が残る場合は該当 `risk_owner` を追加承認者とする。

## 解決済み（M5）

- M5 では claim 抽出を frontmatter または `*.llmwiki.yaml` の claim metadata と explicit claim id に限定し、本文、`## Citations`、段落末尾 citation link からの claim 自動抽出は採用しない。詳細は [ADR 014](./adr/014-finalize-m5-maintenance-contract.md)。
- M5 では contradiction の初期検出対象を structured metadata と明示 relation に限定し、本文意味比較は初期実装に含めない。初期完成の必須範囲から外す判断は [ADR 018](./adr/018-defer-semantic-maintenance-detection.md)。
- M5 では stale claim の初期検出を `review_after` の期限超過に限定し、source 更新に基づく stale 判定は初期完成の必須範囲に含めない。詳細は [ADR 018](./adr/018-defer-semantic-maintenance-detection.md)。

## 実装時に検証する事項

- OKF v0.1 Draft の仕様更新有無。
- 日本語本文と英語識別子が検索・graph 生成に与える影響。
