# Maintenance 仕様

この文書は M5 Maintenance の完了条件を満たすため、graph lint、docs lint、gardening Agent Skill、継続検出の対象を定義する。

## 適用範囲

対象は LLMWiki Core の file-first 運用である。lint は repository 内の Markdown bundle を読み、問題を検出し、人間または Agent が扱える提案を出力する。lint は source、wiki、graph index、workflow state の責務を混ぜない。

## lint policy

lint は既定で修正を行わない。自動修正を許可する場合でも、対象は以下に限定する。

- Markdown link の表記ゆれを、同一 target に解決できる場合に正規化する。
- frontmatter key の順序を、既定順に並べ替える。
- `index.md` から参照されている既存 page の title を再同期する。

以下は自動修正しない。

- stale claim の本文更新。
- contradiction の解消。
- missing citation に対する source の選定。
- owner、reviewer、risk owner の割り当て。
- redaction 判定、generalization 判定、publish 判定。
- lifecycle state の遷移。

## graph lint target

| ID | Target | Detection | Output |
| --- | --- | --- | --- |
| `graph.broken_link` | broken link | Markdown link の target が bundle 内に存在しない | source file、line、target |
| `graph.orphan_page` | orphan page | `index.md` と他 page から参照されない wiki page | page、候補 parent |
| `graph.missing_required_link` | 重要概念の欠落 | requirement、ADR、spec が関連文書を参照していない | page、期待 link 種別 |
| `graph.unknown_relation` | unknown relation | frontmatter または inline metadata の relation が初期語彙にない。Markdown link は正本であり、この finding は補助 relation にのみ適用する | page、relation |
| `graph.ambiguous_relation` | ambiguous relation | 同一 source/target 間に矛盾する relation がある | source、target、relations |
| `graph.superseded_without_target` | superseded target 欠落 | `supersedes` または `superseded_by` が target を持たない | page、relation |

初期 relation vocabulary は Requirement 011 の一覧を採用する。typed relation は補助 metadata であり、lint は Markdown link を graph edge の正本として扱う。

## docs lint target

| ID | Target | Detection | Output |
| --- | --- | --- | --- |
| `docs.missing_frontmatter` | missing frontmatter | reserved files を除く wiki page が required frontmatter を持たない | page、missing keys |
| `docs.invalid_scope` | invalid scope | concept document の scope が `personal`、`team`、`org` 以外 | page、scope |
| `docs.invalid_lifecycle` | invalid lifecycle state | lifecycle が定義済み state 以外 | page、state |
| `docs.missing_owner` | missing owner | published または org candidate の page に owner がない | page |
| `docs.missing_reviewer` | missing reviewer for org policy | org scope または org publish candidate に reviewer がない | page |
| `docs.missing_citation` | missing citation | claim を持つ wiki page に `## Citations` section がない、または claim を支える段落末尾に citation link がない | page、section |
| `docs.stale_claim` | stale claim | `review_after` を過ぎた claim または source が更新された claim | page、claim id、date |
| `docs.duplicate_concept` | duplicated concept | 同一 normalized title または alias が複数 page に存在する | pages、normalized key |
| `docs.contradiction` | contradiction | `contradicts` relation または同一 claim id の不一致 | pages、claim ids |
| `docs.index_log_drift` | index/log drift | page が追加・削除されたが `index.md` または `log.md` に反映されていない | page、expected update |
| `docs.unknown_top_level_key` | unknown top-level key | frontmatter の top-level key が既定 schema になく、`llmwiki` namespace 外にある。read は許容し、lint は warning とする | page、key |

`claim` の機械抽出方式は未決事項である。初期実装では frontmatter の claim metadata、見出し単位の explicit claim id、または human-maintained citation section を手がかりにし、M2 Format で固定した段落単位の citation 表現を検査する。

## gardening Agent Skill

gardening は定期的に lint result を読み、修正案または判断依頼を作る Agent Skill である。

| Skill | Input | Output | Stop Condition |
| --- | --- | --- | --- |
| `lint_graph` | Markdown bundle、relation vocabulary | graph lint report | typed relation 保存方式が必要な場合 |
| `lint_docs` | Markdown bundle、format profile | docs lint report | claim 抽出に人間判断が必要な場合 |
| `detect_conflicts` | citation、claim metadata、relation graph | contradiction report | どちらの claim を採用するか判断が必要な場合 |
| `route_reviewers` | lint report、ownership metadata | reviewer assignment proposal | owner または risk owner が不明な場合 |
| `deprecate_or_link_source_page` | stale/orphan report、page metadata | deprecation/link proposal | lifecycle state を変える判断が必要な場合 |

各 skill は `model: gpt-5.4 medium` の実装フェーズ制約に従う。skill は page を直接 publish しない。org scope に影響する変更は human review に送る。

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

- `graph.broken_link` は `error`。
- `docs.invalid_scope` は `error`。
- `docs.invalid_lifecycle` は `error`。
- `docs.missing_reviewer` は org publish candidate だけ `error`。
- `docs.missing_citation` は published page だけ `error`。
- `docs.unknown_top_level_key` は `warning`。

`stale_claim`、`duplicate_concept`、`contradiction` は初期 gate では `warning` とし、review queue に送る。これは誤検出時に正しい知識更新を止めないためである。

## 未決事項

- typed relation を補助 metadata として保持する場合の schema と保存場所。
- redaction scan の実装方式。
- claim 抽出方式と stale 判定の単位。
- contradiction の自動検出対象を metadata に限定するか、本文要約比較まで広げるか。
