# Format Profile の仕様

## 目的

この文書は、LLMWiki の M2 Format における OKF-compatible Markdown profile を定義する。

対象は wiki store に置く Markdown bundle であり、raw source は対象外とする。raw source は immutable な証拠であり、wiki はその解釈層である。

## 適用範囲

- 適用対象は、`docs/` で管理する知識 bundle 内の Markdown ファイルである。
- 非予約の `.md` ファイルは concept document として扱う。
- 予約ファイルは `index.md` と `log.md` である。
- この profile は OKF v0.1 の最小規約を壊さず、LLMWiki 固有情報を `llmwiki` namespace に閉じ込める。

## 規範ルール

### 1. concept document

- concept document は parseable YAML frontmatter を持たなければならない。
- concept document の frontmatter には、非空の `type` を持たなければならない。
- concept document は `index.md` と `log.md` を除く `.md` ファイルである。

### 2. reserved files

- `index.md` と `log.md` は予約ファイルである。
- 予約ファイルは concept document ではない。
- 予約ファイルは `type` 必須ルールの対象外である。
- 予約ファイルの役割は、bundle の入口と履歴を分離することである。

### 3. frontmatter schema

この profile が固定する最小 schema は次の通りである。

| Key | Required | Type | Constraint |
| --- | --- | --- | --- |
| `type` | yes | string | 非空 |
| `llmwiki` | yes for LLMWiki concept documents | object | LLMWiki 固有 metadata の namespace |
| `llmwiki.scope` | yes for LLMWiki concept documents | string | `personal` / `team` / `org` のいずれか |

- `llmwiki` 配下に置かない top-level key は、OKF の producer-defined key として許容する。
- `llmwiki` 配下の追加 key は producer-defined とし、この profile では個別の意味を固定しない。
- `llmwiki.scope` は document の有効範囲を表す論理ラベルであり、保存場所そのものを意味しない。

### 4. scope 表現

- `personal` は個人の発見、仮説、個人メモを表す。
- `team` は実務に耐える再利用可能な局所知識を表す。
- `org` は組織横断の正規知識、語彙、制約、ポリシー、公式判断を表す。
- scope は `personal -> team -> org` の 3 層で表現する。
- `org` は全文中央集約ではない。scope は意味論であり、中央保存を要求しない。

### 5. Markdown link rule

- bundle 内の関係は Markdown link で表現する。
- 参照先が bundle 内にある場合は relative Markdown link を使う。
- bundle 外の証拠は canonical URL または安定した外部参照先への Markdown link を使う。
- relation を plain text だけで表現せず、graph edge として辿れる形にする。
- broken link は lint 対象とする。

### 6. citation format

- 事実 claim は、追跡可能な citation を持たなければならない。
- citation は `## Citations` section に集約する。
- `## Citations` の各項目は、少なくとも 1 個の Markdown link を持つ。
- 文章中で claim を支える場合は、該当段落の末尾に citation link を置く。
- citation を持たない claim は、未確認の扱いとして confidence を下げるか、記述を保留する。

推奨形式:

```md
## Citations

- [Open Knowledge Format v0.1 SPEC](../references/index.md#open-knowledge-format-v01-spec)
- [Requirement 010: Source Evidence and Citation](../requirements/010-source-evidence-and-citation.md)
```

### 7. index/log rule

- 各 bundle の入口には `index.md` を置く。
- 各 bundle の履歴には `log.md` を置く。
- bundle を構成する directory が 5 件以上の page を持つ場合は、独自の `index.md` と `log.md` を持つ。
- `index.md` は内容指向の navigation を担当し、`log.md` は時系列の変更履歴を担当する。
- `index.md` と `log.md` の更新漏れは lint 対象とする。

### 8. raw source immutable 境界

- raw source は immutable であり、wiki 側で直接編集しない。
- citation は raw source への参照を提供するが、raw source の内容を上書きしない。
- raw source の正規化、要約、抽象化、redaction は、別の派生成果物として扱う。
- wiki document は raw source の解釈層であり、raw source の代替物ではない。

## 未決事項

この profile では、次の点を仕様に固定しない。

- unknown frontmatter key の round-trip 要件を parser の責務として保証するか。
- typed relation を frontmatter に保存するか、別の index に保存するか。
- citation の最小粒度を paragraph、claim sentence、または section のどれに固定するか。
- `index.md` と `log.md` に frontmatter を許可するかどうか。

## 適合性メモ

- 既存の OKF v0.1 最小規約と矛盾しないこと。
- `llmwiki` namespace は producer-defined key として扱えること。
- `index.md` と `log.md` は reserved files として扱えること。
- raw source と wiki の境界を混在させないこと。
