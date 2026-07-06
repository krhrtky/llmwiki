---
type: spec
llmwiki:
  scope: team
  lifecycle: active
---

# Workflow and Access 仕様

## 目的

この文書は、M4 の knowledge lifecycle、propose workflow、redaction/generalization gate、review/approval/rejection、operation-aware access control を結ぶ最小仕様を定義する。
基盤の visibility boundary は [ADR 023](../adr/023-use-storage-registry-for-visibility-boundaries.md) に従い、`private`、`team:<team_id>`、任意の `org` store とする。
この文書で固定するのは、workflow と scope evaluation の実装契約である。認可エンジンの評価順序と競合解決は [ADR 016](../adr/016-finalize-access-policy-evaluation.md) に従う。redaction scan の初期実装方式は [ADR 017](../adr/017-use-rule-based-redaction-scan-initially.md) に従う。
store の直接編集権限は [ADR 024](../adr/024-delegate-store-edit-authorization-to-repository-controls.md) に従い、repository controls に委譲する。

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

`hold` は lifecycle state ではなく、review や scope evaluation の結果として扱う。

## propose draft の構造

`propose` が作る draft は copy ではない。下位 scope のページをそのまま複製せず、上位 scope 向けに再構成した提出物を作る。

### 必須要素

- `source_pages`: 元ページまたは元ページ群。
- `from_store` / `to_store`: 昇格元と昇格先。
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

- `private -> team:<team_id>` と `team:<team_id> -> org` 以外の draft は propose としない。
- `org` store が未設定の場合、`team:<team_id> -> org` は propose としない。
- `reviewer` または `approver` が欠ける draft は validation label として `incomplete` を付ける。`incomplete` は lifecycle state ではない。
- `risk_owner` が必要条件を満たすのに未割当の場合、draft は `incomplete` とし、review に進めない。
- `org` 向け draft は human review がない限り published にしない。

## redaction/generalization gate の責務

gate は propose の必須前段であり、秘匿情報が上位 store に混入しないように止める。

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

- `published` へ遷移した proposal は、元 page と publish page を Markdown 本文で相互に link する。
- 元 page 側には `published_to` 相当の参照を残し、publish page 側には `derived_from` 相当の参照を残す。
- page 間 link の正本は Markdown 本文に置く。implementation traceability は `*.llmwiki.yaml` の `relations[]` に置き、workflow sidecar には実行結果と selection を記録する。

## ownership の責務

| Role | Responsibility | Required When |
| --- | --- | --- |
| `page_owner` | page 単位の正確性、citation、更新期限、lifecycle を管理する。 | `active`、`published`、`proposed` の page。 |
| `domain_owner` | 複数 page にまたがる概念、policy、procedure、decision の整合性を管理する。 | domain 横断の concept、policy、procedure、decision。 |
| `team_owner` | team scope の公開責任と team publish の承認責任を持つ。 | `team` scope へ publish する proposal。 |
| `reviewer` | 内容、根拠、scope、redaction 結果を確認する。 | `team` または `org` への propose。 |
| `approver` | publish の可否を決定する。 | `published` へ遷移する proposal。 |
| `risk_owner` | privacy、security、legal、人事などの risk を判定する。 | sensitive category が検出された proposal、または high-impact operation。 |

`page_owner` は個別 page の保守責任を持つ。`team_owner` は team scope の publish 判断を持つ。`domain_owner` は org を含む複数 page の関係、重複、矛盾、用語統一を扱う。`risk_owner` は redaction/generalization gate の `residual_risk` と operation-aware scope rule の `hold` / `exclude` 判断に接続する。

## operation-aware scope rule schema の構造

access control は visibility ではなく operation 単位で判定する。
初期概念モデルは [ADR 005](../adr/005-use-operation-aware-access-control.md) に従う。評価順序と競合解決は [ADR 016](../adr/016-finalize-access-policy-evaluation.md) に従う。
M4 では scope rule を「最小契約」として固定し、認可エンジンの完全な schema とみなさない。
scope rule は store の直接編集権限を表現しない。direct edit の可否は repository write permission、branch protection、CODEOWNERS、required review、merge permission などの repository controls が決める。

### scope rule

```yaml
scope_rule:
  rule_id: string
  subject:
    kind: user | agent | service_account | role
    id: string
  scope: personal | team | org # 互換 metadata。store 指定時は store から導出する。
  store_id: private | team:<team_id> | org
  team_id: string | null
  operation: read | search | retrieve | query | answer_suggestion | propose | redaction_scan | generalize | lint | graph_build | export | publish | train
  content_level: metadata | summary | content
  resource:
    type: concept_document | policy | procedure | decision | source | graph_index | workflow_state
    selector: string
  selection: include | exclude | hold
  reason: string
  conditions:
    require_human_review: true | false
    require_redaction_gate: true | false
    require_owner: true | false
    require_reviewer: true | false
```

### scope evaluation

scope evaluation は後で説明できる形で残す。

```yaml
scope_evaluation:
  subject: string
  operation: string
  content_level: string
  resource: string
  selection: include | exclude | hold
  rule_ids: [string]
  evaluated_by: string
  evaluated_at: string
  reason: string
```

### fixed minimum audit fields

- `subject`
- `operation`
- `content_level`
- `resource`
- `selection`
- `rule_ids`
- `evaluated_by`
- `evaluated_at`
- `reason`

### 制約

- `metadata`、`summary`、`content` は別の access tier として扱う。
- 複数 team store では `store_id` または `team_id` が不一致の scope rule を一致扱いにしない。
- store の直接編集権限は scope evaluator の責務に含めない。
- scope rule と scope evaluation の最小項目は固定する。実装上の評価順序、優先度、競合解決は [ADR 016](../adr/016-finalize-access-policy-evaluation.md) に従う。
- `export` と `train` は高影響操作として明示的に制御する。
- `hold` は lifecycle state ではなく、workflow sidecar に残す review/scope evaluation とする。scope evaluation はその監査用の派生記録として扱う。

## Related Requirements

- [Requirement 002: Human Agent Responsibility](../requirements/002-human-agent-responsibility.md)
- [Requirement 004: Scope Model](../requirements/004-scope-model.md)
- [Requirement 005: Knowledge Lifecycle](../requirements/005-knowledge-lifecycle.md)
- [Requirement 006: Propose Workflow](../requirements/006-propose-workflow.md)
- [Requirement 007: Redaction and Generalization](../requirements/007-redaction-and-generalization.md)
- [Requirement 008: Operation-Aware Access Control](../requirements/008-operation-aware-access-control.md)
- [Requirement 018: Review and Ownership](../requirements/018-review-and-ownership.md)

## Related ADRs

- [ADR 023: Use Storage Registry for Visibility Boundaries](../adr/023-use-storage-registry-for-visibility-boundaries.md)
- [ADR 003: Use Propose Not Backport](../adr/003-use-propose-not-backport.md)
- [ADR 004: Require Redaction Gate](../adr/004-require-redaction-gate.md)
- [ADR 005: Use Operation-Aware Access Control](../adr/005-use-operation-aware-access-control.md)
- [ADR 012: Require Human Review for Org Publish](../adr/012-require-human-review-for-org-publish.md)
- [ADR 016: Finalize Scope Rule Evaluation](../adr/016-finalize-access-policy-evaluation.md)
- [ADR 017: Use Rule-Based Redaction Scan Initially](../adr/017-use-rule-based-redaction-scan-initially.md)
- [ADR 021: Trace Docs and Implementation with Stable Evidence Links](../adr/021-trace-docs-and-implementation-with-stable-evidence-links.md)
- [ADR 024: Delegate Store Edit Authorization to Repository Controls](../adr/024-delegate-store-edit-authorization-to-repository-controls.md)
- [ADR 025: Rename Access Policy Vocabulary to Scope Rules](../adr/025-rename-access-policy-vocabulary-to-scope-rules.md)
