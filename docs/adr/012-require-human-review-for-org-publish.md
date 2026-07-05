---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 012: Require Human Review for Org Publish

## Status

Accepted

## Context

`org` scope の知識は組織横断で再利用される。Agent が redaction や generalization を行っても、公開判断は人間が行う必要がある。

## Decision

`org` publish には human review を必須にする。

## Alternatives

- Agent publish を許可する: 速度は上がるが、秘匿情報や誤判断の影響が大きい。
- team owner のみで publish する: domain 横断リスクを見落とす可能性がある。
- human review を必須にする: 速度は落ちるが、判断責任を明確にできる。

## Rationale

LLMWiki は人間の判断を置き換えるのではなく、判断材料を外化する仕組みである。org publish は判断そのものなので、人間が承認する。

## Consequences

- Positive: 公開責任と承認責任が明確になる。
- Positive: risk owner を review に参加させられる。
- Negative: propose から publish までの lead time が伸びる。

## Related Requirements

- [Requirement 002](../requirements/002-human-agent-responsibility.md)
- [Requirement 018](../requirements/018-review-and-ownership.md)
