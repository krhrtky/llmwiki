---
type: spec
llmwiki:
  scope: team
  lifecycle: active
---

# Format Profile の仕様

## 目的

この文書は、LLMWiki の M2 Format における OKF-compatible Markdown profile を定義する。

対象は wiki store に置く Markdown bundle であり、raw source は対象外とする。raw source は immutable な証拠であり、wiki はその解釈層である。

## 適用範囲

- 適用対象は、`docs/` で管理する知識 bundle 内の Markdown ファイルである。
- 非予約の `.md` ファイルは concept document として扱う。
- 予約ファイルは `index.md` と `log.md` である。
- `index.md` は bundle の入口として必須である。
- `log.md` は予約ファイルだが必須ではない。履歴を運用する bundle でのみ置く。
- 予約ファイルも parseable YAML frontmatter を持てる。
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
- 予約ファイルの frontmatter は任意であり、存在する場合は parseable YAML でなければならない。
- 予約ファイルの役割は、bundle の入口と履歴を分離することである。

### 3. frontmatter schema

この profile が固定する最小 schema は次の通りである。

| Key | Required | Type | Constraint |
| --- | --- | --- | --- |
| `type` | yes for concept documents | string | 非空 |
| `llmwiki` | yes for LLMWiki concept documents | object | LLMWiki 固有 metadata の namespace |
| `llmwiki.scope` | yes for LLMWiki concept documents | string | `personal` / `team` / `org` のいずれか |

- `llmwiki` 配下に置かない top-level key は、OKF の producer-defined key として許容する。
- `llmwiki` 配下の追加 key は producer-defined とし、この profile では個別の意味を固定しない。
- `llmwiki.scope` は document の有効範囲を表す論理ラベルであり、保存場所そのものを意味しない。
- unknown top-level key は read で許容し、writer は既存の unknown key を破壊してはならない。
- unknown top-level key は lint warning の対象とする。
- docs 間の navigation、根拠参照、bundle 内の graph edge の正本は Markdown link である。
- implementation artifact への traceability の正本は page 隣接 sidecar の `relations[]` であり、typed relation はそこに置く補助 metadata とする。

### 4. scope 表現

- `personal` は個人の発見、仮説、個人メモを表す。
- `team` は実務に耐える再利用可能な局所知識を表す。
- `org` は組織横断の正規知識、語彙、制約、ポリシー、公式判断を表す。
- scope は `personal -> team -> org` の 3 層で表現する。
- `private` は scope に追加しない。個人/private 相当の知識は `personal` とし、秘匿度、公開可否、operation ごとの参照可否は access policy と metadata で扱う。
- `org` は全文中央集約ではない。scope は意味論であり、中央保存を要求しない。
- `personal` の管理主体は作成者個人、`team` の管理主体は対象チーム、`org` の管理主体は組織横断の reviewer または owner とする。
- 下位 scope から上位 scope へ移す場合は、直接 copy せず propose workflow を通す。

### 5. Markdown link rule

- docs 間の navigation、bundle 内の graph edge、根拠参照は Markdown link で表現する。
- 参照先が bundle 内にある場合は relative Markdown link を使う。
- bundle 外の証拠は canonical URL または安定した外部参照先への Markdown link を使う。
- relation を plain text だけで表現せず、graph edge として辿れる形にする。
- implementation artifact への traceability を表す typed relation は page 隣接 sidecar の `relations[]` に置き、Markdown link と責務を分ける。
- broken link は lint 対象とする。

### 6. citation format

- 事実 claim は、追跡可能な citation を持たなければならない。
- citation は `## Citations` section に集約する。
- `## Citations` の各項目は、少なくとも 1 個の Markdown link を持つ。
- citation の最小粒度は段落単位とし、文章中で claim を支える場合は、該当段落の末尾に citation link を置く。
- citation を持たない claim は、未確認の扱いとして confidence を下げるか、記述を保留する。

推奨形式:

```md
## Citations

- [Open Knowledge Format v0.1 SPEC](../references/index.md#open-knowledge-format-v01-spec)
- [Requirement 010: Source Evidence and Citation](../requirements/010-source-evidence-and-citation.md)
```

### 7. index/log rule

- 各 bundle の入口には `index.md` を置く。
- `log.md` は予約ファイルとして許容するが、必須ではない。
- bundle を構成する directory が 5 件以上の page を持つ場合は、独自の `index.md` を持つ。
- `index.md` と `log.md` は frontmatter を持てるが、concept document 扱いにはしない。
- `index.md` は内容指向の navigation を担当し、`log.md` は時系列の変更履歴を担当する。
- `index.md` の更新漏れは lint 対象とする。
- `log.md` の更新漏れは、`log.md` を運用している bundle に限り lint 対象とする。

### 8. raw source immutable 境界

- raw source は immutable であり、wiki 側で直接編集しない。
- citation は raw source への参照を提供するが、raw source の内容を上書きしない。
- raw source の正規化、要約、抽象化、redaction は、別の派生成果物として扱う。
- wiki document は raw source の解釈層であり、raw source の代替物ではない。

## 適合性メモ

- 既存の OKF v0.1 最小規約と矛盾しないこと。
- `llmwiki` namespace は producer-defined key として扱えること。
- `index.md` と `log.md` は reserved files として扱えること。
- raw source と wiki の境界を混在させないこと。

## 既存 SoT docs の初期補完

- 既存の SoT docs を初期登録するときは、`llmwiki.scope: team` と `llmwiki.lifecycle: active` を既定値として補完する。
- `private` は現行 scope には含めず、個人相当の知識は `personal` として扱う。

## Related Requirements

- [Requirement 009](../requirements/009-okf-compatible-format.md)
- [Requirement 010](../requirements/010-source-evidence-and-citation.md)

## Related ADRs

- [ADR 006](../adr/006-adopt-okf-compatible-markdown.md)
- [ADR 007](../adr/007-extend-okf-with-llmwiki-namespace.md)
