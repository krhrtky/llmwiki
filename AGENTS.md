# AGENTS.md

この repository は LLMWiki の Source of Truth と実装基盤を育てるための workspace である。Codex はこのファイルを入口として読み、詳細は `docs/` に進む。

## 基本言語

- ユーザーとのやり取り、ドキュメント本文、判断理由の説明は日本語で行う。
- 識別子、frontmatter key、CLI/API 名、コード内の公開インターフェース名は英語を使う。

## 最初に読む場所

1. `docs/index.md`: SoT の範囲、読む順番、実装制約。
2. `docs/glossary.md`: LLMWiki 固有用語。
3. `docs/requirements/index.md`: 要求と milestone。
4. `docs/adr/index.md`: 採用済み ADR。
5. `docs/open-questions.md`: 未決事項。

AGENTS.md は詳細仕様を保持しない。詳細な要求、ADR、未決論点は `docs/` に記録する。

## コンテキストエンジニアリング

- 実装前に、目的、前提、関連 ADR、未決事項、影響範囲を確認する。
- 会話ログや一時的な推論に依存せず、再利用すべき文脈は `docs/` に外化する。
- 判断材料は人間が読める単位に分ける。長い背景は requirement、技術判断は ADR、未決論点は `docs/open-questions.md` に置く。
- Agent は人間の判断を置き換えない。判断に必要な背景、選択肢、不採用理由、懸念を見える形にする。
- 文脈を圧縮して失うより、引き継ぎ可能な Markdown として残す。

## 実装時の原則

- 実装フェーズは `model: gpt-5.4 medium` で実行する。
- 変更前に関連する requirement と ADR を読む。
- 未決事項を推測で埋めない。実装判断が分岐する場合は、ユーザー確認または追加 ADR を作る。
- 初期方針は file-first、CLI-first。DB、vector DB、外部 service は ADR なしに前提化しない。
- raw source は immutable、wiki は LLM-maintained な解釈層として扱う。
- LLMWiki Core と domain application を混同しない。

## ドキュメント運用

- requirement は WHY と受け入れ条件を中心に書く。
- ADR は Context、Decision、Alternatives、Rationale、Consequences、Open Questions を含める。
- 曖昧な副詞や判断語は、条件、閾値、責務、拒否条件に置き換える。
- 作業状況を成果物に混ぜない。検討中の論点は `docs/open-questions.md` に分離する。
- 一時ファイルや作業ログで Git repository を汚染しない。

## LLMWiki 固有の前提

- visibility boundary は storage registry の `private`、`team:<team_id>`、任意の `org` とする。
- 上位 scope への昇格候補提出は `propose` と呼ぶ。
- `propose` では redaction / generalization gate を必須にする。
- access control は visibility だけでなく operation-aware に扱う。
- wiki store は OKF-compatible Markdown bundle とする。
- LLMWiki 固有 metadata は frontmatter の `llmwiki` 配下に置く。
- `org` publish は human review を必須にする。
