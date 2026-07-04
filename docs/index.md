# LLMWiki Source of Truth

この `docs/` は LLMWiki 実装の Source of Truth である。実装者と Agent は、コードを書く前にここを参照し、未決事項を推測で埋めない。

## 読む順番

1. [glossary.md](./glossary.md) で用語を揃える。
2. [requirements/index.md](./requirements/index.md) で要求と実装順序を確認する。
3. [adr/index.md](./adr/index.md) で採用済みの技術判断を確認する。
4. [open-questions.md](./open-questions.md) で未決事項を確認する。
5. [references/index.md](./references/index.md) で判断材料の出典を確認する。

## SoT の範囲

- LLMWiki の目的、要求、判断理由、実装順序を記録する。
- ADR は採用済みの技術判断と、不採用にした選択肢を記録する。
- requirement は PRD 相当として、実装の WHY と受け入れ条件を記録する。
- 詳細なコード設計、クラス設計、API schema は後続の設計文書または実装計画で扱う。

## 実装制約

- 実装フェーズは `model: gpt-5.4 medium` で実行する。
- ドキュメント本文は日本語で書く。
- 識別子、frontmatter key、CLI/API 名は英語を使う。
- 一時ファイルや作業ログで Git リポジトリを汚染しない。

## 参照関係

- requirement は関連 ADR へリンクする。
- ADR は根拠となる requirement と参照元へリンクする。
- 未決事項は本文に混ぜず、[open-questions.md](./open-questions.md) に集約する。
