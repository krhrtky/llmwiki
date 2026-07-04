# Maintenance 仕様

この文書は M5 Maintenance の初期実装を deterministic lint に固定し、graph lint、docs lint、gardening Agent Skill、継続検出の対象を定義する。

## 適用範囲

対象は LLMWiki Core の file-first 運用である。lint は repository 内の Markdown bundle を読み、問題を検出し、人間または Agent が扱える提案を出力する。初期 M5 は deterministic lint を対象にし、本文意味の推論や LLM 判定を前提にしない。lint は source、wiki、graph index、workflow state の責務を混ぜない。

## M5 初期実装範囲

- 初期 M5 は deterministic lint を対象にする。
- 初期実装の必須範囲に自動修正は含めない。
- lint の基本動作は検出と提案であり、修正は別工程に分離する。

## lint policy

lint は既定で修正を行わない。初期実装では自動修正を必須範囲に含めない。lint は検出と提案を基本とし、修正は人間または別の実行段階が行う。

以下は自動修正しない。

- stale claim の本文更新。
- contradiction の解消。
- missing citation に対する source の選定。
- owner、reviewer、risk owner の割り当て。
- redaction 判定、generalization 判定、publish 判定。
- lifecycle state の遷移。

## 判定対象

初期実装の判定対象は以下に固定する。

- published page: `llmwiki.lifecycle: published`
- org publish candidate: page の場合は `llmwiki.scope: org` かつ `llmwiki.lifecycle: proposed | reviewing`。proposal の場合は workflow state の `to_scope: org` かつ `lifecycle: proposed | reviewing`
- active page: `llmwiki.lifecycle: active | published`

M5 lint は proposal workflow state の状態 field を `lifecycle` として読む。`state` は M5 の判定 field として使わない。

## graph lint target

| ID | Target | Detection | Output |
| --- | --- | --- | --- |
| `graph.broken_link` | broken link | Markdown link の target が bundle 内に存在しない | source file、line、target |
| `graph.orphan_page` | orphan page | `index.md` と他 page から参照されない wiki page | page、候補 parent |
| `graph.missing_required_link` | 重要概念の欠落 | requirement、ADR、spec が required link を持たない | page、期待 link 種別 |
| `graph.unknown_relation` | unknown relation | `*.llmwiki.yaml` の relation が初期語彙にない。Markdown link は正本であり、この finding は補助 relation にのみ適用する | page、relation |
| `graph.ambiguous_relation` | ambiguous relation | `*.llmwiki.yaml` の同一 source/target 間に複数の relation type がある | source、target、relations |
| `graph.superseded_without_target` | superseded target 欠落 | `*.llmwiki.yaml` の `supersedes` または `superseded_by` が target を持たない | page、relation |

初期 relation vocabulary は Requirement 011 の一覧を採用する。typed relation は補助 metadata であり、lint は Markdown link を graph edge の正本として扱う。

### typed relation sidecar schema

typed relation の補助 metadata は `*.llmwiki.yaml` に保存する。最小 schema は次の構造に固定する。

```yaml
owner: string
reviewer: string
risk_owner: string
claims:
  - claim_id: string
    review_after: YYYY-MM-DD
    # optional
    value: string
relations:
  - type: depends_on
    target: docs/example.md
```

`value` は optional とし、top-level `relations[]` は `type` と `target` のみを持つ。lint は top-level `relations[]` を relation 入力として読み、frontmatter、本文、`page.workflow.yaml`、または `relations[]` 以外の metadata は graph relation 判定の入力にしない。

`graph.unknown_relation`、`graph.ambiguous_relation`、`graph.superseded_without_target` はこの sidecar relation を入力にする。`docs.contradiction` も explicit `contradicts` relation をこの sidecar から読む。

### `graph.missing_required_link` の required link matrix

| Source type | Required section | Required link target |
| --- | --- | --- |
| requirement | `## Related ADRs` | 1 件以上の ADR または spec への Markdown link |
| ADR | `## Related Requirements` | 1 件以上の requirement への Markdown link |
| spec | `## Related Requirements` または `## Related ADRs` | 対応 requirement または ADR への Markdown link |

required link は指定 section 内の Markdown link のみを数える。本文中の任意 link や意味的に関連しそうな文は required link として扱わない。spec が required section を持たない場合は、同一 directory の `index.md` から当該 spec への link があり、かつ当該 spec 本文に対応 requirement または ADR への Markdown link がある場合に限り warning を抑制できる。

## docs lint target

| ID | Target | Detection | Output |
| --- | --- | --- | --- |
| `docs.missing_frontmatter` | missing frontmatter | reserved files を除く wiki page が required frontmatter を持たない | page、missing keys |
| `docs.invalid_scope` | invalid scope | concept document の scope が `personal`、`team`、`org` 以外 | page、scope |
| `docs.invalid_lifecycle` | invalid lifecycle state | lifecycle が定義済み state 以外 | page、state |
| `docs.missing_owner` | missing owner | published page または org publish candidate の page に owner がない | page |
| `docs.missing_reviewer` | missing reviewer for org policy | org scope または org publish candidate に reviewer がない | page |
| `docs.missing_citation` | missing citation | published page に `## Citations` section がない、または claim を支える段落末尾に citation link がない | page、section |
| `docs.stale_claim` | stale claim | `review_after` を過ぎた structured claim | page、claim id、date |
| `docs.duplicate_concept` | duplicated concept | 同一 normalized title または alias が複数 page に存在する | pages、normalized key |
| `docs.contradiction` | contradiction | `*.llmwiki.yaml` の `contradicts` relation または同一 claim id の不一致 | pages、claim ids |
| `docs.index_log_drift` | index/log drift | page が追加・削除されたが `index.md` または `log.md` に反映されていない | page、expected update |
| `docs.unknown_top_level_key` | unknown top-level key | frontmatter の top-level key が既定 schema になく、`llmwiki` namespace 外にある。read は許容し、lint は warning とする | page、key |

### claim 検出の初期範囲

`claim` の機械抽出は、frontmatter または `*.llmwiki.yaml` にある次の明示構造のみを対象にする。

- `claim_id`
- `review_after`
- citation metadata

`## Citations` と段落末尾の Markdown citation link は citation 検査の入力であり、claim 抽出の入力ではない。本文から意味抽出はしない。段落本文の要約比較や LLM による claim 同定は初期範囲外である。

`docs.stale_claim` の初期検出は `review_after` の期限超過だけを対象にする。source 更新との比較には source revision、timestamp、snapshot id などの metadata contract が必要なため、初期 M5 では検出対象に含めない。

### contradiction 検出の初期範囲

`docs.contradiction` の初期検出は、次の明示構造のみを対象にする。

- explicit `contradicts` relation
- 同一 `claim_id` の構造化 metadata 不一致

本文要約比較や LLM 意味比較は対象外である。

## gardening Agent Skill

gardening は定期的に lint result を読み、修正案または判断依頼を作る Agent Skill である。

| Skill | Input | Output | Stop Condition | 禁止事項 |
| --- | --- | --- | --- | --- |
| `lint_graph` | Markdown bundle、relation vocabulary、required link matrix | graph lint report | parse failure、または graph model が確定できない場合 | page を修正しない、lifecycle を変更しない |
| `lint_docs` | Markdown bundle、format profile、判定対象定義 | docs lint report | parse failure、または structured claim metadata が読めない場合 | claim を推測で補完しない、publish 判断をしない |
| `detect_conflicts` | citation、claim metadata、relation graph | contradiction report | explicit `contradicts` relation か claim_id の構造化 metadata が不足する場合 | 本文要約比較を行わない、採否判断をしない |
| `route_reviewers` | lint report、ownership metadata、lifecycle metadata | reviewer assignment proposal | owner または reviewer が決められない場合 | reviewer を自動確定しない、lifecycle を変更しない |
| `deprecate_or_link_source_page` | stale/orphan report、page metadata、graph relations | deprecation/link proposal | source page の候補が一意に定まらない場合 | publish しない、lifecycle を遷移させない |

各 skill は `model: gpt-5.4 medium` の実装フェーズ制約に従う。skill は page を直接 publish しない。org scope に影響する変更は human review に送る。

`deprecate_or_link_source_page` は deprecate または link の提案のみを返し、`published` や `deprecated` への実行を行わない。

### gardening skill JSON contract

gardening skill の正式 output は JSON とする。各 skill は `lint_report.findings[]` の `id`、`path`、`line`、`requires_human_decision` を入力として扱い、page 本文、sidecar、workflow state を直接更新しない。

```json
{
  "gardening_result": {
    "skill": "route_reviewers",
    "generated_at": "2026-07-04T00:00:00Z",
    "inputs": {
      "lint_report_ref": "lint-report.json",
      "finding_ids": ["docs.missing_reviewer"]
    },
    "proposals": [
      {
        "type": "reviewer_assignment",
        "target_path": "docs/example.md",
        "reason": "org publish candidate has no reviewer",
        "requires_human_decision": true,
        "suggested_action": "domain_owner が reviewer を割り当てる"
      }
    ],
    "status": "ok"
  }
}
```

`status` は `ok`、`hold`、`error` の 3 値とする。`hold` は owner、reviewer、risk owner、source page、または採否判断が一意に決められない場合に返す。`error` は parse failure や入力 JSON の不正など技術的失敗に限る。

## report format

lint report の正式 output contract は JSON とする。Markdown report を出す場合でも、同じ JSON 構造から派生生成する。

```json
{
  "lint_report": {
    "generated_at": "2026-07-04T00:00:00Z",
    "bundle": ".",
    "findings": [
      {
        "id": "docs.missing_citation",
        "severity": "error",
        "path": "docs/example.md",
        "line": 42,
        "message": "citation section is missing",
        "requires_human_decision": true,
        "suggested_action": "source_curator が根拠 source を選定する"
      }
    ]
  }
}
```

`severity` は `error`、`warning`、`info` の 3 値とする。`requires_human_decision: true` の finding は自動修正対象外である。

## CI gate

CI に載せる場合の初期 gate は以下に限定する。

- `parse failure` は `error`。
- `graph.broken_link` は `error`。
- `docs.missing_frontmatter` は concept document だけ `error`。
- `docs.invalid_scope` は `error`。
- `docs.invalid_lifecycle` は `error`。
- `docs.missing_citation` は published page だけ `error`。
- `docs.missing_reviewer` は org publish candidate だけ `warning`。
- `docs.missing_owner` は `warning`。
- `docs.stale_claim` は `warning`。
- `docs.duplicate_concept` は `warning`。
- `docs.contradiction` は `warning`。
- `graph.missing_required_link` は `warning`。
- `graph.orphan_page` は `warning`。
- `graph.unknown_relation` は `warning`。
- `graph.ambiguous_relation` は `warning`。
- `graph.superseded_without_target` は `warning`。
- `docs.index_log_drift` は `warning`。
- `docs.unknown_top_level_key` は `warning`。

owner、reviewer、stale、duplicate、contradiction は初期 CI では warning とし、review queue に送る。これは誤検出時に正しい知識更新を止めないためである。

## 未決事項

- redaction scan の実装方式。
- 本文意味比較による contradiction / stale 検出の実装方式。
- source 更新に基づく stale claim 検出の metadata contract。

## Related Requirements

- [Requirement 011: Graph and Relation](../requirements/011-graph-and-relation.md)
- [Requirement 016: Lint and Gardening](../requirements/016-lint-and-gardening.md)
- [Requirement 017: Harness Engineering](../requirements/017-harness-engineering.md)
- [Requirement 020: Implementation Milestones](../requirements/020-implementation-milestones.md)

## Related ADRs

- [ADR 010: Use Index and Log for Progressive Disclosure](../adr/010-use-index-and-log-for-progressive-disclosure.md)
- [ADR 011: Start with File and CLI First](../adr/011-start-with-file-and-cli-first.md)
- [ADR 014: Finalize M5 Maintenance Contract](../adr/014-finalize-m5-maintenance-contract.md)
- [ADR 015: Store Typed Relations in LLMWiki Sidecar](../adr/015-store-typed-relations-in-llmwiki-sidecar.md)
