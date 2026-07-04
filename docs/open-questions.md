# Open Questions

このファイルは未決事項を集約する。実装者は未決事項を推測で埋めず、実装判断が分岐する場合はユーザー判断または追加 ADR を作成する。

## 後続 ADR 候補

- operation-aware access control の policy schema と評価順序。
- access decision の監査ログ項目。
- graph edge を Markdown link の周辺文脈から推定するか、frontmatter に typed relation として保持するか。
- CLI を単一 binary にするか、複数 subcommand package に分けるか。
- metadata store と workflow state store に何を採用するか。
- redaction scan の実装方式。ルールベース、LLM、DLP サービス、または組み合わせ。

## ユーザー判断待ち

- 初期ユースケースを個人 wiki、チーム wiki、問い合わせ管理、またはハーネスエンジニアリング基盤のどれに寄せるか。
- `org` publish の承認者を誰にするか。
- 秘匿情報の最終分類を社内ポリシーと合わせるか、LLMWiki 独自の初期分類で始めるか。

## 実装時に検証する事項

- OKF v0.1 Draft の仕様更新有無。
- `index.md` に frontmatter を持たせる場合の OKF conformance 影響。
- Markdown parser が未知 frontmatter key を round-trip できるか。
- 日本語本文と英語識別子が検索・graph 生成に与える影響。
