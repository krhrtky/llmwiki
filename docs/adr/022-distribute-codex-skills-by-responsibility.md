---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 022: Distribute LLMWiki as a Codex Skill

## Status

Accepted

## Context

LLMWiki は repository-local docs と CLI を SoT とするが、Agent が毎回それらを手動で探索すると入口が不安定になる。この repository から Codex Skill として配布できる導線がないと、他の Agent や別 workspace で同じ作業境界を再利用しにくい。

## Decision

この repository は `skills/*/SKILL.md` を配布可能な Codex Skill suite として持つ。配布導線は `llmwiki skill install --workspace-root . [--codex-home <path>]` とし、`$CODEX_HOME/skills/<skill-name>` または `~/.codex/skills/<skill-name>` に各 skill を配置する。

docs 側は skill contract の正本であり、skill 実体は Agent が最初に読む入口、用途別の CLI 導出手順、停止条件を提供する。詳細な要求、ADR、仕様は skill に複製せず、`docs/` へ progressive disclosure で辿らせる。

## Alternatives

- skill を AGENTS.md に集約する: 入口は単純だが、内容が肥大化しやすい。
- skill を docs だけに集約する: Codex Skill として配布できない。
- marketplace plugin を先に作る: 配布範囲は広がるが、初期段階では manifest と marketplace 運用が増えすぎる。

## Rationale

最小の Codex Skill を repository 内に置くと、LLMWiki の操作境界を他の Agent が再利用できる。skill に詳細仕様を詰め込まないことで、docs SoT と progressive disclosure の方針を保てる。install command を CLI に持たせることで、手作業 copy ではなく検証可能な配布導線にできる。

## Consequences

- Positive: この repository から Codex Skill を配布できる。
- Positive: skill は入口と停止条件に集中し、詳細仕様は docs に残せる。
- Positive: `llmwiki skill install` を testable な配布 command として扱える。
- Negative: skill 実体と docs の入口情報の整合管理が必要になる。

## Related Requirements

- [Requirement 012](../requirements/012-agent-skills.md)
- [Requirement 017](../requirements/017-harness-engineering.md)
- [Requirement 020](../requirements/020-implementation-milestones.md)
