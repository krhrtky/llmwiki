# Workflow and Access 仕様

## 目的

この文書は、M4 の knowledge lifecycle、propose workflow、redaction/generalization gate、review/approval/rejection、operation-aware access control を結ぶ最小仕様を定義する。
基盤の scope モデルは [ADR 002](../adr/002-adopt-personal-team-org-scope.md) に従い、`personal → team → org` とする。
この文書で固定するのは、workflow と access decision の実装契約である。認可エンジンの評価順序と競合解決は [ADR 016](../adr/016-finalize-access-policy-evaluation.md) に従う。redaction scan の初期実装方式は [ADR 017](../adr/017-use-rule-based-redaction-scan-initially.md) に従う。

## lifecycle 状態

knowledge page と proposal は同じ状態語彙を共有するが、意味は異なる。

| State | Meaning |
| --- | --- |
| `draft` | 作成中。まだ scope 内の安定知識として扱わない。 |
| `active` | scope 内で利用可能。 |
| `proposed` | 上位 scope への提案候補。 |
| `reviewing` | reviewer または approver が確認中。 |
| `published` | 上位 scope で公開済み。 |
| `deprecated` | 廃止済み。削除ではなく追跡する。 |
| `rejected` | 提案却下。再提案の出発点になりうる。 |

### 基本遷移

- `draft -> active`
- `active -> proposed`
- `proposed -> reviewing`
- `reviewing -> published`
- `reviewing -> rejected`
- `active/published -> deprecated`
- `rejected -> draft/proposed` は再提案時に許容する

`hold` は lifecycle state ではなく、review や policy decision の結果として扱う。

## propose draft の構造

`propose` が作る draft は copy ではない。下位 scope のページをそのまま複製せず、上位 scope 向けに再構成した提出物を作る。

### 必須要素

- `source_pages`: 元ページまたは元ページ群。
- `from_scope` / `to_scope`: 昇格元と昇格先。
- `evidence`: 参照した source と citation。
- `generalization_notes`: 抽象化した点。
- `redaction_report`: 検出結果、変換方針、残リスク。
- `diff_summary`: 元ページとの差分要約。
- `reviewer`: 確認者。
- `approver`: 公開承認者。

### 条件付き必須要素

- `risk_owner`: `redaction_report.residual_risk` が空でない場合、または sensitive category が検出された場合に必須とする。
- `publish_links`: publish 後に元 page と publish page を相互参照する link plan。`source_page -> published_page` と `published_page -> source_page` の両方向を含める。

### draft の判定

- `to_scope` が上位でない draft は propose としない。
- `reviewer` または `approver` が欠ける draft は validation label として `incomplete` を付ける。`incomplete` は lifecycle state ではない。
- `risk_owner` が必要条件を満たすのに未割当の場合、draft は `incomplete` とし、review に進めない。
- `org` 向け draft は human review がない限り published にしない。

## redaction/generalization gate の責務

gate は propose の必須前段であり、秘匿情報が上位 scope に混入しないように止める。

### gate の役割

- sensitive category を検出する。
- raw text copy を避け、抽象化・匿名化・要約を優先する。
- 残リスクを report に残す。
- 一般化できない場合は `hold` または `rejected` に送る。

### 初期 sensitive category

- 人事情報
- 個人情報
- 顧客固有情報
- 契約情報
- 認証情報
- 未公開事業情報

### gate の出力

- `findings`
- `transformations`
- `residual_risk`
- `blocked_items`
- `recommendation`: `allow` / `hold` / `deny`

gate の出力契約は固定する。初期実装は [ADR 017](../adr/017-use-rule-based-redaction-scan-initially.md) に従い、rule-based deterministic scan とする。LLM、DLP service、または hybrid scan は後続 ADR で扱う。

`recommendation` が `deny` または `hold` の場合、`publish` は行わない。

## review、approval、rejection

- `reviewer` は内容、根拠、redaction、scope を確認する。
- `approver` は publish の可否を決定する human role である。
- `reject` の理由は proposal に記録し、再提案時に参照できるようにする。
- `hold` は再作業が必要な状態として扱い、拒否理由と未完了条件を残す。
- `org` publish は [ADR 012](../adr/012-require-human-review-for-org-publish.md) に従い human review を必須にする。

### approval の条件

- `team` publish の `approver` は対象 team の `team_owner` を基本とし、必要に応じて `domain_owner` へ委譲できる。
- `org` publish の `approver` は human の `domain_owner` を基本とし、sensitive category が残る場合は該当カテゴリの `risk_owner` を追加承認者とする。
- Agent は proposal の準備、redaction report の生成、review 材料の整理までは行えるが、`org` publish の最終承認者にはなれない。

### publish 後の link

- `published` へ遷移した proposal は、元 page と publish page を相互に link する。
- 元 page 側には `published_to` 相当の参照を残し、publish page 側には `derived_from` 相当の参照を残す。
- link の正本は page 本文または page metadata が持つ page 間参照であり、workflow sidecar には実行結果を記録する。

## ownership の責務

| Role | Responsibility | Required When |
| --- | --- | --- |
| `page_owner` | page 単位の正確性、citation、更新期限、lifecycle を管理する。 | `active`、`published`、`proposed` の page。 |
| `domain_owner` | 複数 page にまたがる概念、policy、procedure、decision の整合性を管理する。 | domain 横断の concept、policy、procedure、decision。 |
| `team_owner` | team scope の公開責任と team publish の承認責任を持つ。 | `team` scope へ publish する proposal。 |
| `reviewer` | 内容、根拠、scope、redaction 結果を確認する。 | `team` または `org` への propose。 |
| `approver` | publish の可否を決定する。 | `published` へ遷移する proposal。 |
| `risk_owner` | privacy、security、legal、人事などの risk を判定する。 | sensitive category が検出された proposal、または high-impact operation。 |

`page_owner` は個別 page の保守責任を持つ。`team_owner` は team scope の publish 判断を持つ。`domain_owner` は org を含む複数 page の関係、重複、矛盾、用語統一を扱う。`risk_owner` は redaction/generalization gate の `residual_risk` と operation-aware policy の `hold` / `deny` 判断に接続する。

## operation-aware policy schema の構造

access control は visibility ではなく operation 単位で判定する。
初期概念モデルは [ADR 005](../adr/005-use-operation-aware-access-control.md) に従う。評価順序と競合解決は [ADR 016](../adr/016-finalize-access-policy-evaluation.md) に従う。
M4 では policy object を「最小契約」として固定し、認可エンジンの完全な schema とみなさない。

### policy object

```yaml
policy:
  policy_id: string
  subject:
    kind: user | agent | service_account | role
    id: string
  scope: personal | team | org
  operation: read | search | retrieve | query | answer_suggestion | propose | redaction_scan | generalize | lint | graph_build | export | publish | train
  content_level: metadata | summary | content
  resource:
    type: concept_document | policy | procedure | decision | source | graph_index | workflow_state
    selector: string
  decision: allow | deny | hold
  reason: string
  conditions:
    require_human_review: true | false
    require_redaction_gate: true | false
    require_owner: true | false
    require_reviewer: true | false
```

### decision log

policy decision は後で説明できる形で残す。

```yaml
decision_log:
  subject: string
  operation: string
  content_level: string
  resource: string
  decision: allow | deny | hold
  policy_ids: [string]
  decided_by: string
  decided_at: string
  reason: string
```

### fixed minimum audit fields

- `subject`
- `operation`
- `content_level`
- `resource`
- `decision`
- `policy_ids`
- `decided_by`
- `decided_at`
- `reason`

### 制約

- `metadata`、`summary`、`content` は別の access tier として扱う。
- policy object と decision log の最小項目は固定する。実装上の評価順序、優先度、競合解決は [ADR 016](../adr/016-finalize-access-policy-evaluation.md) に従う。
- `export` と `train` は高影響操作として明示的に制御する。
- `hold` は lifecycle state ではなく、workflow sidecar に残す review/policy decision とする。decision log はその監査用の派生記録として扱う。

## 未決事項

- fixed minimum audit fields を超えて追加する必須監査項目。
- LLM または DLP service を redaction scan に追加する条件。
