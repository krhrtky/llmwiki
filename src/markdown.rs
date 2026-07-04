use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use serde_yaml::Value;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MarkdownDocument {
    pub frontmatter: Option<Value>,
    pub body: String,
    pub links: Vec<MarkdownLink>,
    pub headings: Vec<MarkdownHeading>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownLink {
    pub target: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownHeading {
    pub level: u8,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownParseError {
    InvalidFrontmatter(String),
}

pub fn parse_markdown(content: &str) -> Result<MarkdownDocument, MarkdownParseError> {
    let (frontmatter, body, body_start_line) = split_frontmatter(content)?;
    let (mut links, headings) = parse_markdown_events(&body);
    add_link_lines(&body, body_start_line, &mut links);

    Ok(MarkdownDocument {
        frontmatter,
        body,
        links,
        headings,
    })
}

pub fn is_reserved_file(path: &std::path::Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md" | "log.md")
    )
}

pub fn is_external_or_anchor_link(target: &str) -> bool {
    let trimmed = target.trim();
    trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("ftp://")
        || trimmed.starts_with("ssh://")
        || trimmed.starts_with("mailto:")
        || trimmed.contains("://")
}

pub fn resolve_markdown_target(source_path: &std::path::Path, target: &str) -> Option<PathBuf> {
    if is_external_or_anchor_link(target) {
        return None;
    }

    let without_fragment = target.split('#').next().unwrap_or(target);
    let without_anchor = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    if without_anchor.is_empty() {
        return None;
    }

    source_path
        .parent()
        .map(|parent| normalize_path(&parent.join(without_anchor)))
}

pub fn has_citations_section(document: &MarkdownDocument) -> bool {
    document
        .headings
        .iter()
        .any(|heading| heading.level == 2 && heading.text.trim() == "Citations")
}

pub fn citations_section_has_markdown_link(document: &MarkdownDocument) -> bool {
    let mut in_citations = false;
    let mut saw_item = false;

    for line in document.body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_citations = trimmed.trim_start_matches('#').trim() == "Citations";
            continue;
        }
        if in_citations && trimmed.starts_with('#') {
            break;
        }
        if in_citations && trimmed.starts_with('-') {
            saw_item = true;
            if !trimmed.contains("](") {
                return false;
            }
        }
    }

    saw_item
}

pub fn has_paragraph_without_trailing_citation(document: &MarkdownDocument) -> bool {
    let mut in_code_block = false;

    document.body.split("\n\n").any(|paragraph| {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            return false;
        }
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            return false;
        }
        if in_code_block
            || trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with('|')
            || trimmed.starts_with('>')
        {
            return false;
        }

        !trimmed.ends_with(')') || !trimmed.contains("](")
    })
}

fn split_frontmatter(content: &str) -> Result<(Option<Value>, String, usize), MarkdownParseError> {
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") && content.trim() != "---" {
        return Ok((None, content.to_string(), 1));
    }

    let mut lines = content.lines();
    let _opening = lines.next();
    let mut yaml = Vec::new();
    let mut body = Vec::new();
    let mut found_closing = false;
    let mut body_start_line = 1;

    for (index, line) in lines.by_ref().enumerate() {
        if line.trim_end_matches('\r') == "---" {
            found_closing = true;
            body_start_line = index + 3;
            break;
        }
        yaml.push(line.trim_end_matches('\r'));
    }

    if found_closing {
        body.extend(lines);
    } else {
        yaml.clear();
        body.clear();
    }

    if !found_closing {
        return Err(MarkdownParseError::InvalidFrontmatter(
            "frontmatter closing marker is missing".to_string(),
        ));
    }

    let parsed = serde_yaml::from_str::<Value>(&yaml.join("\n"))
        .map_err(|source| MarkdownParseError::InvalidFrontmatter(source.to_string()))?;

    Ok((Some(parsed), body.join("\n"), body_start_line))
}

fn parse_markdown_events(body: &str) -> (Vec<MarkdownLink>, Vec<MarkdownHeading>) {
    let parser = Parser::new_ext(body, Options::ENABLE_TABLES);
    let mut links = Vec::new();
    let mut headings = Vec::new();
    let mut active_heading: Option<(u8, String)> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                links.push(MarkdownLink {
                    target: dest_url.to_string(),
                    line: 0,
                });
            }
            Event::Start(Tag::Heading { level, .. }) => {
                active_heading = Some((heading_level(level), String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, heading_text)) = active_heading.as_mut() {
                    heading_text.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, text)) = active_heading.take() {
                    headings.push(MarkdownHeading { level, text });
                }
            }
            _ => {}
        }
    }

    (links, headings)
}

pub fn add_link_lines(body: &str, body_start_line: usize, links: &mut [MarkdownLink]) {
    let link_pattern = Regex::new(r"\[[^\]]+\]\((?P<target>[^)\s]+)").expect("valid regex");
    let mut pending = links.iter_mut().filter(|link| link.line == 0);

    for (line_index, line) in body.lines().enumerate() {
        for capture in link_pattern.captures_iter(line) {
            if capture.name("target").is_some() {
                if let Some(link) = pending.next() {
                    link.line = body_start_line + line_index;
                }
            }
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(segment) => normalized.push(segment),
        }
    }

    normalized
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_links() {
        let parsed = parse_markdown("---\ntype: note\n---\n# Title\n[Ref](a.md)").unwrap();

        assert!(parsed.frontmatter.is_some());
        assert_eq!(parsed.links[0].target, "a.md");
        assert_eq!(parsed.links[0].line, 5);
        assert_eq!(parsed.headings[0].text, "Title");
    }

    #[test]
    fn detects_link_in_citations_section() {
        let parsed = parse_markdown(
            "---\ntype: note\n---\n# Title\n\n## Citations\n\n- [Source](source.md)",
        )
        .unwrap();

        assert!(citations_section_has_markdown_link(&parsed));
    }

    #[test]
    fn requires_link_in_each_citations_item() {
        let parsed = parse_markdown(
            "---\ntype: note\n---\n# Title\n\n## Citations\n\n- [Source](source.md)\n- Plain source",
        )
        .unwrap();

        assert!(!citations_section_has_markdown_link(&parsed));
    }

    #[test]
    fn parses_crlf_frontmatter() {
        let parsed = parse_markdown(
            "---\r\ntype: note\r\nllmwiki:\r\n  scope: personal\r\n---\r\n# Title\r\n",
        )
        .unwrap();

        assert!(parsed.frontmatter.is_some());
        assert_eq!(parsed.headings[0].text, "Title");
    }

    #[test]
    fn detects_markdown_link_in_citations_section() {
        let parsed = parse_markdown(
            "---\ntype: note\n---\n# Title\n\n## Citations\n\n- [Source](source.md)\n",
        )
        .unwrap();

        assert!(citations_section_has_markdown_link(&parsed));
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let error = parse_markdown("---\ntype: note\n# Title").unwrap_err();

        assert_eq!(
            error,
            MarkdownParseError::InvalidFrontmatter(
                "frontmatter closing marker is missing".to_string()
            )
        );
    }
}
