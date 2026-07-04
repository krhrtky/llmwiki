# 未決事項

このファイルは未決事項を集約する。実装者は未決事項を推測で埋めず、実装判断が分岐する場合はユーザー判断または追加 ADR を作成する。

## 後続 ADR 候補

- operation-aware access control の policy schema と評価順序。
- access decision の監査ログ項目。
- typed relation を補助 metadata として保持する場合の schema と保存場所。
- CLI を単一 binary にするか、複数 subcommand package に分けるか。
- metadata store と workflow state store に何を採用するか。
- redaction scan の実装方式。ルールベース、LLM、DLP サービス、または組み合わせ。
- `hold` を lifecycle state に昇格させるか、policy/review decision の結果に留めるか。
- CLI/API output の transport 表現を JSON のみに固定するか、Markdown report も正式形式に含めるか。

## ユーザー判断待ち

- 初期ユースケースを個人 wiki、チーム wiki、問い合わせ管理、またはハーネスエンジニアリング基盤のどれに寄せるか。
- `org` publish の承認者を誰にするか。
- 秘匿情報の最終分類を社内ポリシーと合わせるか、LLMWiki 独自の初期分類で始めるか。

## 実装時に検証する事項

- OKF v0.1 Draft の仕様更新有無。
- 日本語本文と英語識別子が検索・graph 生成に与える影響。
- claim 抽出方式と stale 判定の単位。
- contradiction の自動検出対象を metadata に限定するか、本文要約比較まで広げるか。
