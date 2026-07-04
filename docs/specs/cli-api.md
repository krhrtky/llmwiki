# CLI/API 仕様

## 目的

この文書は、M3 の `CLI/API` に対する最小の操作契約を定義する。
`CLI` と `API` は transport が違うだけで、同じ意味の操作を提供する。
初期実装は [ADR 011](../adr/011-start-with-file-and-cli-first.md) に従い、file-first を前提にする。

## 共通境界

- すべての操作はローカルの workspace root または bundle root を入力に取る。
- `source store` は immutable とし、操作は raw source を上書きしない。
- `wiki store`、`graph index`、`workflow state` は workspace 管理下のファイルとして扱う。
- `metadata store` は page owner、reviewer、scope、lifecycle、citation metadata、access policy reference を保持する file-first store として扱う。
- DB、vector DB、外部 service は必須にしない。
- domain application の case、customer、assignment などは扱わない。
- パスは workspace root の外に出ないことを前提とし、外部パスは拒否する。

## 共通入力

| 項目 | 意味 |
| --- | --- |
| `workspace_root` | 解析対象の bundle root。必須。 |
| `scope` | `personal` / `team` / `org` のいずれか。必要な操作のみ指定する。 |
| `paths` | 対象ファイルまたはページ群。省略時は `workspace_root` 配下全体。 |
| `content_level` | `metadata` / `summary` / `content`。アクセス制御で使用する。 |
| `format` | 出力形式。transport 依存の表現はここでは固定しない。 |

## 操作

| Command | Purpose | Input | Output | Failure conditions | File-first boundary |
| --- | --- | --- | --- | --- | --- |
| `ingest` | raw source から wiki 更新候補を作る。 | `workspace_root`, `paths`, `scope`。必要なら source type。 | 更新候補ページ、citation、差分候補、取り込み結果。 | source が読めない、形式未対応、scope 外入力、raw source を上書きしようとした場合。 | raw source は読み取りのみ。生成物は wiki candidate として別領域へ出す。 |
| `query` | wiki を読んで回答する。 | `workspace_root`, `question`, `scope`, `content_level`。 | 回答、citations、confidence、filing candidate metadata。 | アクセス拒否、根拠不足、入力が write side effect を要求した場合。 | 読み取り専用。query 自体は wiki store を更新しない。 |
| `file` | query や ingest の結果を wiki 更新候補として整理する。 | `workspace_root`, `candidate`, `scope`, `owner`, `citations`。 | filing artifact、required metadata、review queue entry。 | citation 不足、scope 不明、owner 未指定、access policy により保存不可の場合。 | wiki 本体を直接更新せず、review または lint 対象の candidate として保存する。 |
| `lint` | 矛盾、古さ、欠落、broken link を検査する。 | `workspace_root`, `paths`, `scope`。 | lint findings、severity、対象 path。 | parse 不可、bundle 不整合、対象が読めない場合。 | 読み取り専用。修正は別操作に分離する。 |
| `graph` | Markdown link と relation から graph index を作る。 | `workspace_root`, `paths`。 | graph index、edge list、orphan/broken link findings。 | parse 不可、対象が読めない、graph 生成先を workspace 外に置こうとした場合。 | 生成物は derived index のみ。raw source と wiki を分離する。 |
| `propose` | 下位 scope から上位 scope へ提出する draft を作る。 | `workspace_root`, `paths`, `from_scope`, `to_scope`, `reviewer`, `approver`。 | proposal draft、diff summary、evidence map、redaction report 参照。 | 上位への昇格でない、redaction/generalization が未完了、reviewer/approver 未指定、`org` 向けの human review なし。 | proposal draft は source の copy ではなく、別の workflow state として保存する。 |
| `redact` | 秘匿情報を検出し、redaction report を作る。 | `workspace_root`, `paths`, `target_scope`。 | 検出結果、変換方針、残リスク、sanitized draft。 | 検出不能、一般化不能、残リスクが許容できない場合。 | raw source は変更せず、別の sanitized artifact を作る。 |
| `export` | bundle または scope を出力する。 | `workspace_root`, `paths` または `scope`, `content_level`。 | export artifact、manifest、対象一覧。 | 参照不可の内容を含む、未対応形式、workspace 外への出力。 | export は複製物を作るだけで、原本は変えない。 |

## 出力の扱い

- `query` の filing は `file` で扱う。query は保存を自動実行しない。
- `propose` は publish ではない。publish は workflow 側で review 後に扱う。
- `redact` と `propose` は連続して使えるが、役割は分離する。redact は検出と変換、propose は提出用 draft の構成である。

## filing artifact の構造

`file` が作る artifact は、wiki 本文へ反映する前の候補である。

| Field | Required | Meaning |
| --- | --- | --- |
| `source` | yes | query answer、raw source、proposal などの発生元。 |
| `scope` | yes | `personal`、`team`、`org` のいずれか。 |
| `confidence` | yes | 根拠の強さ。`high`、`medium`、`low` の 3 値。 |
| `citations` | yes | 根拠への Markdown link。 |
| `owner` | yes | page owner 候補。 |
| `reviewer` | required for `team` and `org` | review 担当者。 |
| `risk_owner` | required when sensitive category exists | privacy、security、legal、人事などの判断者。 |
| `lifecycle` | yes | 初期値は `draft`。 |
| `access_policy_refs` | yes | 適用する policy の参照。 |

## Agent Skill 接続

Requirement 012 の初期 skill は、CLI/API command と次のように接続する。

| Skill | Command | Input | Output | Stop Condition |
| --- | --- | --- | --- | --- |
| `ingest_source` | `ingest` | raw source path、scope | wiki candidate、citation candidate | source が immutable 境界を越える場合 |
| `triage_knowledge` | `file` | candidate、scope、citations | filing artifact | owner または citation が不明な場合 |
| `nominate_for_promotion` | `propose` | active page、from_scope、to_scope | proposal seed | to_scope が上位でない場合 |
| `propose_knowledge` | `propose` | source_pages、reviewer、approver | proposal draft | redaction/generalization が未完了の場合 |
| `generalize_for_upper_scope` | `redact` / `propose` | lower-scope content、target_scope | generalized draft | 抽象化で意味が失われる場合 |
| `redact_sensitive_information` | `redact` | paths、target_scope | redaction report、sanitized draft | 残リスクが許容できない場合 |
| `map_evidence` | `ingest` / `file` | candidate、source links | evidence map | 根拠が追跡できない場合 |
| `detect_conflicts` | `lint` | paths、claim metadata | contradiction findings | 採否判断が必要な場合 |
| `route_reviewers` | `file` / `propose` | filing artifact、ownership metadata | reviewer assignment proposal | owner または risk owner が不明な場合 |
| `build_graph` | `graph` | workspace_root、paths | graph index、edge list | link parse が失敗する場合 |
| `lint_graph` | `lint` / `graph` | graph index、bundle | graph lint findings | relation 保存方式の判断が必要な場合 |
| `promote_knowledge` | `propose` | approved proposal | publish candidate | human review が不足する場合 |
| `deprecate_or_link_source_page` | `lint` / `file` | stale/orphan report | deprecation/link proposal | lifecycle state 変更の判断が必要な場合 |

## 失敗の表現

- `deny`: policy により実行不可。
- `hold`: その場での実行は止めるが、再評価余地がある。
- `error`: 入力不正、parse failure、I/O failure などの技術的失敗。

`hold` は workflow や policy decision として扱い、必ずしも lifecycle の最終状態ではない。

## 未決事項

- 出力の transport 形式を JSON のみに固定するかどうかは未決事項とする。
- graph edge の正本は Markdown link とし、typed relation を補助 metadata として保持する場合の schema と保存場所は [docs/open-questions.md](../open-questions.md) に従う。
