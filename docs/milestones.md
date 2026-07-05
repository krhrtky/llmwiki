---
type: milestone_index
llmwiki:
  scope: team
  lifecycle: active
---

# Milestone 完了トラッカー

この文書は Requirement 020 の M1 から M5 までについて、完了条件と証跡を一箇所に集約する。

## 完了条件

| Milestone | Completion State | Evidence |
| --- | --- | --- |
| M1 SoT | Complete | `docs/index.md`、`docs/glossary.md`、`docs/requirements/`、`docs/adr/`、`docs/open-questions.md` |
| M2 Format | Complete | `docs/specs/format-profile.md` |
| M3 CLI/API | Complete | `docs/specs/cli-api.md`、`docs/adr/013-finalize-m3-cli-contract.md` |
| M4 Workflow | Complete | `docs/specs/workflow-and-access.md` |
| M5 Maintenance | Complete | `docs/specs/maintenance.md`、`docs/adr/014-finalize-m5-maintenance-contract.md`、`docs/open-questions.md` |

具体仕様の一覧は [仕様索引](./specs/index.md) を参照する。

## Requirement 対応

| Milestone | Primary Requirements | Completion Evidence |
| --- | --- | --- |
| M1 SoT | 001, 002, 003, 017, 020 | SoT entrypoint、glossary、requirements index、ADR index、open questions |
| M2 Format | 004, 009, 010, 011 | scope、OKF-compatible Markdown、citation、graph link rule |
| M3 CLI/API | 012, 013, 014, 015 | Agent Skill 接続、CLI/API command contract、storage boundary、query/filing、JSON output contract、sidecar layout |
| M4 Workflow | 005, 006, 007, 008, 018 | lifecycle、propose、redaction gate、operation-aware access、review/ownership |
| M5 Maintenance | 011, 016, 017, 020 | graph lint、docs lint、gardening Agent Skill、CI gate、後続論点の分離 |

## ADR 対応

| Milestone | ADRs |
| --- | --- |
| M1 SoT | ADR 001, ADR 010 |
| M2 Format | ADR 006, ADR 007, ADR 009, ADR 010 |
| M3 CLI/API | ADR 008, ADR 011, ADR 013 |
| M4 Workflow | ADR 002, ADR 003, ADR 004, ADR 005, ADR 012 |
| M5 Maintenance | ADR 001, ADR 010, ADR 011, ADR 014 |

## 完了ルール

- 完了判定は、本文の存在ではなく、対応 requirement の受け入れ条件を満たす証跡があることで行う。
- 未決事項は完了を妨げない。ただし仕様分岐が必要な点は `docs/open-questions.md` に集約し、各 spec の `Open Questions` は該当項目の再掲に留める。
- 実装フェーズの制約 `model: gpt-5.4 medium` は AGENTS.md、`docs/index.md`、Requirement 012、Requirement 020 に記録されている。
- docs↔implementation traceability と Codex Skill 配布の方針は、ADR 021、ADR 022、および関連 requirement/spec で追跡する。
- domain application 固有の workflow、DB、外部 service は milestone 完了条件に含めない。
