# Workflow and Access 仕様

## 目的

この文書は、M4 の knowledge lifecycle、propose workflow、redaction/generalization gate、review/approval/rejection、operation-aware access control を結ぶ最小仕様を定義する。
基盤の scope モデルは [ADR 002](../adr/002-adopt-personal-team-org-scope.md) に従い、`personal → team → org` とする。

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

### draft の判定

- `to_scope` が上位でない draft は propose としない。
- `reviewer` または `approver` が欠ける draft は validation label として `incomplete` を付ける。`incomplete` は lifecycle state ではない。
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

`recommendation` が `deny` または `hold` の場合、`publish` は行わない。

## review、approval、rejection

- `reviewer` は内容、根拠、redaction、scope を確認する。
- `approver` は publish の可否を決定する。
- `reject` の理由は proposal に記録し、再提案時に参照できるようにする。
- `hold` は再作業が必要な状態として扱い、拒否理由と未完了条件を残す。
- `org` publish は [ADR 012](../adr/012-require-human-review-for-org-publish.md) に従い human review を必須にする。

## ownership の責務

| Role | Responsibility | Required When |
| --- | --- | --- |
| `page_owner` | page 単位の正確性、citation、更新期限、lifecycle を管理する。 | `active`、`published`、`proposed` の page。 |
| `domain_owner` | 複数 page にまたがる概念、policy、procedure、decision の整合性を管理する。 | domain 横断の concept、policy、procedure、decision。 |
| `reviewer` | 内容、根拠、scope、redaction 結果を確認する。 | `team` または `org` への propose。 |
| `approver` | publish の可否を決定する。 | `published` へ遷移する proposal。 |
| `risk_owner` | privacy、security、legal、人事などの risk を判定する。 | sensitive category が検出された proposal、または high-impact operation。 |

`page_owner` は個別 page の保守責任を持つ。`domain_owner` は複数 page の関係、重複、矛盾、用語統一を扱う。`risk_owner` は redaction/generalization gate の `residual_risk` と operation-aware policy の `hold` / `deny` 判断に接続する。

## operation-aware policy schema の構造

access control は visibility ではなく operation 単位で判定する。
初期概念モデルは [ADR 005](../adr/005-use-operation-aware-access-control.md) に従い、評価順序の確定は後続 ADR に残す。

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

### 制約

- `metadata`、`summary`、`content` は別の access tier として扱う。
- policy schema は概念固定までに留め、実装上の評価順序、優先度、競合解決は未決事項とする。
- `export` と `train` は高影響操作として明示的に制御する。

## 未決事項

- policy の優先順位と評価順序。
- decision log に含める監査項目の最小集合。
- redaction scan の実装方式。
- `hold` を workflow state に昇格させるかどうか。
