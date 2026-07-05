# References

## Harness Engineering

- [LLM Wiki](https://gist.githubusercontent.com/karpathy/442a6bf555914893e9891c11519de94f/raw/ac46de1ad27f92b28ac95459c782c07f6b8c964a/llm-wiki.md)
  - raw source、wiki、schema の 3 層に分ける。
  - raw source は immutable、wiki は LLM-maintained とする。
  - `index.md` と `log.md` は navigation と履歴の基本構造になる。
  - LLMWiki の価値は retrieval ではなく maintenance にある。
- [OpenAI Harness Engineering](https://openai.com/index/harness-engineering/)
  - AGENTS.md を百科事典にせず、repository-local docs を SoT とする。
  - progressive disclosure により Agent が必要な情報へ段階的に辿れるようにする。
  - architecture、taste、quality rule は機械的に検証可能にする。
  - 人間はコードを書くよりも、意図、環境、feedback loop、判断を設計する。
- [Anthropic Harness Design](https://www.anthropic.com/engineering/harness-design-long-running-apps)
  - 長時間実行される Agent には、検証可能な環境と明確な作業境界が必要。
  - Agent が自律的に進める範囲と、人間判断が必要な範囲を分離する。
- [Martin Fowler Harness Engineering](https://martinfowler.com/articles/harness-engineering.html)
  - Agent の出力品質は harness の設計に強く依存する。
  - repository 内に知識、検証、実行手順を外化することで再現性を高める。

## Format and Provenance

- [Open Knowledge Format v0.1 SPEC](https://raw.githubusercontent.com/GoogleCloudPlatform/knowledge-catalog/main/okf/SPEC.md)
  - Knowledge Bundle は Markdown file tree。
  - concept document は YAML frontmatter を持つ。
  - 必須 frontmatter は `type` のみ。
  - `index.md` と `log.md` は予約ファイル。
  - 追加 frontmatter key は producer-defined として許容される。
  - consumer は未知 type、未知 key、broken link を拒否しない。
- [OKF 解説](https://dev.classmethod.jp/articles/open-knowledge-format-okf-v01-guide/)
  - OKF は AGENTS.md や LLM wiki pattern を形式化する方向性を持つ。
  - platform ではなく format として扱うのが妥当。
  - MCP や AGENTS.md と競合せず、静的知識の表現として補完する。

## Traceability and Provenance Graph

- [SpeakerDeck AIE2026](https://speakerdeck.com/visional_engineering_and_design/aie2026)
  - 2026年6月8日・9日の AI Engineering Summit Tokyo 2026 登壇資料である。
  - Authority / Specification / Provenance Graph を分離する。
  - SSOT と derived data を分離する。
  - 「書かれている」より「効いている」を優先する。
