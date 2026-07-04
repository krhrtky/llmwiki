# References

## LLM Wiki

- URL: https://gist.githubusercontent.com/karpathy/442a6bf555914893e9891c11519de94f/raw/ac46de1ad27f92b28ac95459c782c07f6b8c964a/llm-wiki.md
- 抽出した判断材料:
  - raw source、wiki、schema の 3 層に分ける。
  - raw source は immutable、wiki は LLM-maintained とする。
  - `index.md` と `log.md` は navigation と history の基本構造になる。
  - LLMWiki の価値は retrieval ではなく maintenance にある。

## Open Knowledge Format v0.1 SPEC

- URL: https://raw.githubusercontent.com/GoogleCloudPlatform/knowledge-catalog/main/okf/SPEC.md
- 抽出した判断材料:
  - Knowledge Bundle は Markdown file tree。
  - concept document は YAML frontmatter を持つ。
  - 必須 frontmatter は `type` のみ。
  - `index.md` と `log.md` は予約ファイル。
  - 追加 frontmatter key は producer-defined として許容される。
  - consumer は未知 type、未知 key、broken link を拒否しない。

## OKF 解説

- URL: https://dev.classmethod.jp/articles/open-knowledge-format-okf-v01-guide/
- 抽出した判断材料:
  - OKF は AGENTS.md や LLM wiki pattern を形式化する方向性を持つ。
  - platform ではなく format として扱うのが妥当。
  - MCP や AGENTS.md と競合せず、静的知識の表現として補完する。

## OpenAI Harness Engineering

- URL: https://openai.com/index/harness-engineering/
- 抽出した判断材料:
  - AGENTS.md を百科事典にせず、repository-local docs を SoT とする。
  - progressive disclosure により Agent が必要な情報へ段階的に辿れるようにする。
  - architecture、taste、quality rule は機械的に検証可能にする。
  - 人間はコードを書くよりも、意図、環境、feedback loop、判断を設計する。

## Anthropic Harness Design

- URL: https://www.anthropic.com/engineering/harness-design-long-running-apps
- 抽出した判断材料:
  - 長時間実行される Agent には、検証可能な環境と明確な作業境界が必要。
  - Agent が自律的に進める範囲と、人間判断が必要な範囲を分離する。

## Martin Fowler Harness Engineering

- URL: https://martinfowler.com/articles/harness-engineering.html
- 抽出した判断材料:
  - Agent の出力品質は harness の設計に強く依存する。
  - repository 内に知識、検証、実行手順を外化することで再現性を高める。
