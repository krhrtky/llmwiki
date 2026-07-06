---
type: spec
llmwiki:
  scope: team
  lifecycle: active
---

# Related Retrieval 仕様

## 目的

この文書は、grep を超えて関連知識を辿る retrieval の Post-M5 仕様を外化する。初期の固定 CLI/API は `llmwiki related` とし、`llmwiki retrieve` は lexical seed selection、section chunk、BM25 などを要する後続経路として残す。初期実装の正本は OKF-compatible Markdown、`*.llmwiki.yaml`、derived graph index であり、DB、vector DB、外部 service は必須にしない。

## 基本フロー

related retrieval は次の順序で実行する。

1. query intent を決める。
2. seed page を明示入力として受け取る。
3. seed から relation graph を辿る。
4. 選択された store root の内側だけを traversal 対象にする。
5. 各 traversal step で operation-aware scope rule を適用する。
6. relation weight、distance、confidence、freshness、sensitivity で rerank する。
7. context と explain path を返す。

## Retrieval Intent

| Intent | 優先 relation | 用途 |
| --- | --- | --- |
| `answer_suggestion` | `answers`, `constrained_by`, `depends_on`, `decided_by`, `derived_from`, `implemented_by`, `verified_by` | 回答案の根拠、制約、手順を取得する。 |
| `impact_analysis` | reverse `implements`, reverse `implemented_by`, reverse `constrained_by`, reverse `depends_on`, `supersedes`, `enforced_by` | policy や ADR 変更の影響範囲を取得する。 |
| `propose` | `similar_to`, `specializes`, `contradicts`, `supersedes`, `derived_from`, `constrained_by`, `distributed_as`, `verified_by` | 上位 scope へ提出する前に重複、矛盾、秘匿リスクを確認する。 |
| `global_summary` | entity graph, community summary | org-level sensemaking と propose candidate discovery に使う。通常 retrieval の置き換えにしない。 |

## Relation Metadata

`*.llmwiki.yaml` の `relations[]` は、初期 lint の必須項目として `type` と `target` を持つ。retrieval では optional metadata を読める。

```yaml
relations:
  - type: constrained_by
    target: ../policy/identity-verification.md
    target_kind: doc
    provenance: human
    confidence: high
    status: active
```

`target_kind` は target の種別を示す補助情報で、`doc`、`code`、`test`、`skill`、`command`、`generated`、`external` を基本とする。Markdown target では省略時に `doc` とみなす。`provenance` は `human`、`llm`、`embedding`、`parser` を基本とする。`status` は `active`、`proposed`、`deprecated` を基本とする。`similar_to` と `mentions` は recall 向上のために使い、公式な依存や制約として扱わない。

## Access Filter

access check は最後だけでなく traversal の各段階で行う。

- seed node
- neighbor edge
- neighbor node
- section body

`metadata` は graph traversal に使える場合がある。`summary` と `content` は operation、actor、scope、scope evaluation に従って取得可否を分ける。

## Scoring

初期 scoring は deterministic にする。

```text
final_score =
  lexical_score
  + relation_weight
  + confidence_weight
  - distance_penalty
  - sensitivity_penalty
```

初期 relation weight は次を既定値にする。

| Relation | Weight |
| --- | ---: |
| `constrained_by` | 1.00 |
| `enforced_by` | 0.90 |
| `decided_by` | 0.95 |
| `answers` | 0.90 |
| `depends_on` | 0.85 |
| `implements` | 0.80 |
| `implemented_by` | 0.80 |
| `derived_from` | 0.75 |
| `specializes` | 0.65 |
| `verified_by` | 0.55 |
| `mentions` | 0.40 |
| `similar_to` | 0.30 |
| `distributed_as` | 0.25 |
| `related_to` | 0.15 |

`enforced_by`、`implemented_by`、`verified_by`、`distributed_as` は docs↔implementation traceability と skill distribution のための relation として扱う。`enforced_by` は policy、guardrail、CI gate の強制を、`implemented_by` は requirement、spec、ADR の実装証跡を表す。`verified_by` は test、lint、review の検証を表し、`distributed_as` は skill や role skill の分配を表す。

## 初期 CLI/API 契約

初期の固定 command は `llmwiki related` とする。seed page を明示入力し、relation traversal のみを担当させる。section seed、free-form query から seed を選ぶ lexical search は `llmwiki retrieve` の後続責務とし、この段階では固定しない。

```bash
llmwiki related --workspace-root . --scope team docs/procedure.md \
  --operation answer_suggestion \
  --content-level content \
  --subject-kind user \
  --subject-id alice \
  --retrieval-scope scope-rules/team-query.yaml \
  --depth 2 \
  --limit 10
```

`llmwiki related` の CLI options は次を基本とする。

| Option | Required | Meaning |
| --- | --- | --- |
| `workspace_root` | yes | 解析対象 bundle root。 |
| `config` | preferred | storage registry。 |
| `store` | preferred | `private` / `team:<team_id>` / `org` のいずれか。 |
| `scope` | compatibility | `personal` / `team` / `org` のいずれか。store 指定時は導出する。 |
| `seed` | yes | 関連展開の起点となる page path。section fragment は初期実装では受け付けない。 |
| `operation` | yes | `answer_suggestion` / `impact_analysis` / `propose`。`global_summary` は後続 adapter として扱う。 |
| `content_level` | yes | `metadata` / `summary` / `content`。 |
| `subject_kind` | yes | `user` / `agent` / `service_account` / `role`。 |
| `subject_id` | yes | access check 対象の subject id。 |
| `retrieval_scope` | yes | retrieval scope YAML。複数指定できる。 |
| `depth` | no | relation traversal の最大深さ。初期値は `2`。 |
| `limit` | no | 返す結果数の上限。初期値は `10`。 |
| `format` | no | 初期実装では `json` のみを正式サポートする。 |

正式 output は JSON とし、結果本文だけでなく relation path と scope evaluation を返す。外側の envelope は `related_result` とする。

```json
{
  "related_result": {
    "status": "success",
    "seed": "docs/procedure/scout-delivery.md",
    "operation": "answer_suggestion",
    "scope": "team",
    "content_level": "content",
    "depth": 2,
    "results": [
      {
        "path": "docs/policy/identity-verification.md",
        "score": 0.91,
        "content": "...",
        "relation_paths": [
          [
            {
              "from": "docs/procedure/scout-delivery.md",
              "relation": "constrained_by",
              "to": "docs/policy/identity-verification.md",
              "source": "typed_relation",
              "direction": "forward"
            }
          ]
        ],
        "scope_evaluations": [
          {
            "stage": "seed",
            "log": {
              "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
              "operation": "answer_suggestion",
              "content_level": "metadata",
              "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/procedure/scout-delivery.md\"}",
              "selection": "include",
              "rule_ids": ["team-answer-content"],
              "evaluated_by": "scope_evaluator",
              "evaluated_at": "2026-07-06T10:15:30Z",
              "reason": "seed page is inside the selected team store and metadata access is allowed"
            }
          },
          {
            "stage": "edge",
            "log": {
              "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
              "operation": "answer_suggestion",
              "content_level": "metadata",
              "resource": "{\"type\":\"relation_edge\",\"selector\":\"docs/procedure/scout-delivery.md --constrained_by--> docs/policy/identity-verification.md\"}",
              "selection": "include",
              "rule_ids": ["team-answer-content"],
              "evaluated_by": "scope_evaluator",
              "evaluated_at": "2026-07-06T10:15:30Z",
              "reason": "relation traversal metadata is allowed for the selected store"
            }
          },
          {
            "stage": "neighbor",
            "log": {
              "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
              "operation": "answer_suggestion",
              "content_level": "metadata",
              "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/policy/identity-verification.md\"}",
              "selection": "include",
              "rule_ids": ["team-answer-content"],
              "evaluated_by": "scope_evaluator",
              "evaluated_at": "2026-07-06T10:15:31Z",
              "reason": "neighbor page metadata is available within the same team store"
            }
          },
          {
            "stage": "section_body",
            "log": {
              "subject": "{\"kind\":\"user\",\"id\":\"alice\"}",
              "operation": "answer_suggestion",
              "content_level": "content",
              "resource": "{\"type\":\"concept_document\",\"selector\":\"docs/policy/identity-verification.md\"}",
              "selection": "include",
              "rule_ids": ["team-answer-content"],
              "evaluated_by": "scope_evaluator",
              "evaluated_at": "2026-07-06T10:15:31Z",
              "reason": "content retrieval is allowed for this page under the selected retrieval scope"
            }
          }
        ],
        "why": "seed page が constrained_by relation で policy に依存しているため"
      }
    ],
    "scope_evaluations": []
  }
}
```

### Access Check Stages

scope evaluation は最終出力だけでなく traversal の各段階で行う。
store selector を使う場合、traversal は選択 store の canonical root に閉じる。cross-store link は取得対象ではなく evidence または blocked reference として扱う。

- seed node
- neighbor edge
- neighbor node
- section body

各 stage は `include` / `exclude` / `hold` と理由を返す。seed node が `exclude` または `hold` の場合は command 全体を止める。neighbor edge、neighbor node、section body が `exclude` または `hold` の場合は、その枝を候補から除外する。

### Scoring and Sort

`llmwiki related` の初期 scoring は deterministic にする。明示 seed を入力にするため、lexical score は `0` とする。`confidence_weight` と `sensitivity_penalty` は optional metadata と感度分類の固定後に有効化し、初期実装では `0` とする。

```text
final_score =
  relation_weight
  - distance_penalty
```

初期 `distance_penalty` は hop ごとに `0.10` とする。結果は `final_score` の降順で並べ、同点は `distance` の昇順、`path` の昇順、`relation_paths` の文字列表現の昇順で決める。

## 後続 Adapter

- `llmwiki retrieve` は free-form query からの lexical seed selection、section seed、section chunk、BM25、embedding を扱う後続 command とし、`related` の初期契約とは分離する。
- PostgreSQL + pgvector: page、section、edge、embedding を同じ DB で扱う候補。
- OpenSearch / Elasticsearch: BM25、日本語 analyzer、検索運用を強める候補。
- Neo4j: graph traversal と可視化が主要 UI になった場合の derived graph index 候補。
- GraphRAG: org-level sensemaking、community summary、propose candidate discovery の候補。

これらは derived index adapter であり、Markdown bundle と sidecar の SoT を置き換えない。

## Related Requirements

- [Requirement 011: Graph and Relation](../requirements/011-graph-and-relation.md)
- [Requirement 015: Query and Filing](../requirements/015-query-and-filing.md)
- [Requirement 008: Operation-Aware Access Control](../requirements/008-operation-aware-access-control.md)

## Related ADRs

- [ADR 011: Start with File and CLI First](../adr/011-start-with-file-and-cli-first.md)
- [ADR 015: Store Typed Relations in LLMWiki Sidecar](../adr/015-store-typed-relations-in-llmwiki-sidecar.md)
- [ADR 020: Use Hybrid Search and Graph Traversal for Related Retrieval](../adr/020-use-hybrid-search-and-graph-traversal-for-related-retrieval.md)
