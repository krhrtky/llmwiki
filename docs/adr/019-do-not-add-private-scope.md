---
type: adr
llmwiki:
  scope: team
  lifecycle: active
---

# ADR 019: Do Not Add Private Scope

## Status

Superseded by [ADR 023](./023-use-storage-registry-for-visibility-boundaries.md)

## Context

ADR 002 は scope を `personal → team → org` に固定している。一方で、個人メモや秘匿性の高い知識を `private` scope として追加する案が残っていた。

scope は知識の有効範囲を表す。秘匿度、公開可否、operation ごとの参照可否は access policy と metadata の責務であり、scope に混ぜると propose workflow と validation が複雑になる。

## Decision

`private` は独立 scope として追加しない。個人/private 相当の知識は `personal` scope として扱う。

秘匿度、公開可否、operation ごとの参照可否は、operation-aware access control、sidecar metadata、redaction gate で扱う。

## Alternatives

- `private` を `personal` より下位 scope として追加する: scope rank と propose workflow が 4 層になり、現行 CLI validation と ADR 002 を更新する必要がある。
- `private` を `personal` の別名として許容する: 同じ意味の scope 名が複数になり、lint、export、query、propose の contract が曖昧になる。
- `private` を visibility として追加する: scope と visibility の責務分離は保てるが、現行 access policy の用語と重複する。

## Rationale

`personal` は個人の発見、仮説、個人メモを扱えるため、private 相当の知識を表現できる。秘匿性は有効範囲ではなくアクセス制御の問題である。

scope を 3 層に保つことで、`personal → team → org` の propose rank、lint、export、query の契約を維持できる。

## Consequences

- Positive: scope validation を `personal`、`team`、`org` に固定できる。
- Positive: private 相当の知識を access policy と redaction gate で扱う責務が明確になる。
- Positive: ADR 002 の scope model を維持できる。
- Negative: `private` という語を使いたい利用者には、`personal` と access policy の組み合わせを説明する必要がある。
- Negative: sensitivity metadata の詳細 contract は後続仕様で具体化する必要がある。

## Related Requirements

- [Requirement 004](../requirements/004-scope-model.md)
- [Requirement 008](../requirements/008-operation-aware-access-control.md)
- [Requirement 007](../requirements/007-redaction-and-generalization.md)
