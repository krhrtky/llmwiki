use std::fs;
use std::process::Command as ProcessCommand;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn llmwiki() -> Command {
    Command::cargo_bin("llmwiki").expect("llmwiki binary")
}

#[test]
fn init_creates_a_wiki_that_passes_check() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");

    llmwiki()
        .args(["init", target.to_str().expect("UTF-8 target")])
        .assert()
        .success();

    llmwiki()
        .args(["check", target.to_str().expect("UTF-8 target")])
        .assert()
        .success();
    assert!(target.join("raw/.gitkeep").is_file());
    assert!(target.join(".llmwiki/manifest.json").is_file());
}

#[test]
fn source_add_is_idempotent_and_preflights_name_collisions() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    let source = temporary.path().join("note.txt");
    let other = temporary.path().join("other.txt");
    fs::write(&source, "A useful raw note").expect("source");
    fs::write(&other, "other").expect("source");
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();

    llmwiki()
        .args([
            "source",
            "add",
            target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .success();
    llmwiki()
        .args([
            "source",
            "add",
            target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .success();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(target.join(".llmwiki/manifest.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["sources"]["raw/note.txt"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );

    fs::write(&source, "different content").expect("changed source");
    llmwiki()
        .args([
            "source",
            "add",
            target.to_str().unwrap(),
            other.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .code(2);
    assert!(!target.join("raw/other.txt").exists());
    assert_eq!(
        fs::read_to_string(target.join("raw/note.txt")).unwrap(),
        "A useful raw note"
    );
}

#[test]
fn source_add_requires_at_least_one_file() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();

    llmwiki()
        .args(["source", "add", target.to_str().unwrap()])
        .assert()
        .code(2);
}

#[test]
fn source_add_preserves_pdf_and_png_bytes() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    let pdf = temporary.path().join("scan.pdf");
    let png = temporary.path().join("scan.png");
    fs::write(&pdf, b"%PDF-1.7\0immutable").unwrap();
    fs::write(&png, b"\x89PNG\r\n\x1a\n\0immutable").unwrap();
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    llmwiki()
        .args([
            "source",
            "add",
            target.to_str().unwrap(),
            pdf.to_str().unwrap(),
            png.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert_eq!(
        fs::read(target.join("raw/scan.pdf")).unwrap(),
        fs::read(pdf).unwrap()
    );
    assert_eq!(
        fs::read(target.join("raw/scan.png")).unwrap(),
        fs::read(png).unwrap()
    );
}

#[test]
fn search_uses_full_unicode_casefold_and_skips_binary_raw() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    write_page(
        &target,
        "concepts/search",
        "Search Title",
        "concept",
        "Straße と 東京",
        "sources: [\"[[sources/evidence]]\"]\n",
    );
    fs::write(target.join("raw/notes.txt"), "STRASSE and 東京").unwrap();
    fs::write(target.join("raw/image.png"), b"STRASSE\0").unwrap();

    let output = llmwiki()
        .args(["search", target.to_str().unwrap(), "strasse"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("wiki/concepts/search.md:9"));
    assert!(output.contains("raw/notes.txt:1"));
    assert!(!output.contains("image.png"));
    llmwiki()
        .args(["search", target.to_str().unwrap(), "東京"])
        .assert()
        .success();
}

#[test]
fn check_validates_provenance_links_and_json_diagnostics() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    let source = temporary.path().join("report.pdf");
    fs::write(&source, b"%PDF-test").unwrap();
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    llmwiki()
        .args([
            "source",
            "add",
            target.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .assert()
        .success();
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(target.join(".llmwiki/manifest.json")).unwrap())
            .unwrap();
    let digest = manifest["sources"]["raw/report.pdf"]["sha256"]
        .as_str()
        .unwrap();
    write_page(
        &target,
        "sources/report",
        "Report",
        "source",
        "![[raw/report.pdf|Original]]\n\n## Summary\n\nSummary.\n\n## Claims\n\nClaim.",
        &format!("source_file: raw/report.pdf\nsource_sha256: {digest}\n"),
    );
    write_page(
        &target,
        "concepts/derived",
        "Derived",
        "concept",
        "[[sources/report#Claims|Evidence]]",
        "sources: [\"[[sources/report|Report]]\"]\n",
    );
    fs::write(
        target.join("wiki/index.md"),
        "# Wiki Index\n\n[[sources/report]]\n[[concepts/derived]]\n",
    )
    .unwrap();
    llmwiki()
        .args(["check", target.to_str().unwrap(), "--format", "json"])
        .assert()
        .success();

    fs::write(target.join("raw/report.pdf"), b"changed").unwrap();
    fs::write(
        target.join("wiki/concepts/Bad Name.md"),
        "# no frontmatter\n",
    )
    .unwrap();
    let output = llmwiki()
        .args(["check", target.to_str().unwrap(), "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    let codes = output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["code"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"raw-hash-mismatch"));
    assert!(codes.contains(&"invalid-filename"));
    assert!(output["summary"]["errors"].as_u64().unwrap() > 0);
}

#[test]
fn check_reports_broken_ambiguous_and_noncanonical_links() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    write_page(
        &target,
        "concepts/one",
        "One",
        "concept",
        "[[missing]]\n[[one]]\n[[wiki/concepts/one.md]]",
        "sources: [\"[[sources/missing]]\"]\n",
    );
    write_page(
        &target,
        "entities/one",
        "Other",
        "entity",
        "",
        "sources: [\"[[sources/missing]]\"]\n",
    );
    fs::write(
        target.join("wiki/index.md"),
        "# Wiki Index\n\n[[concepts/one]]\n[[entities/one]]\n",
    )
    .unwrap();
    let output = llmwiki()
        .args(["check", target.to_str().unwrap(), "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    let codes = output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["code"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"broken-link"));
    assert!(codes.contains(&"ambiguous-link"));
    assert!(codes.contains(&"noncanonical-link"));
}

#[test]
fn check_reports_manifest_raw_frontmatter_index_orphan_and_log_violations() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    let registered = temporary.path().join("registered.txt");
    fs::write(&registered, "registered").unwrap();
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    llmwiki()
        .args([
            "source",
            "add",
            target.to_str().unwrap(),
            registered.to_str().unwrap(),
        ])
        .assert()
        .success();
    fs::remove_file(target.join("raw/registered.txt")).unwrap();
    fs::write(target.join("raw/manual.txt"), "not registered").unwrap();
    let manifest_path = target.join(".llmwiki/manifest.json");
    let mut manifest: Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["sources"]["raw/missing.txt"] = Value::String("not an entry".to_owned());
    manifest["sources"]["not-raw.txt"] = serde_json::json!({"sha256": "not-a-hash"});
    fs::write(manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();
    fs::write(target.join("wiki/concepts/Bad Name.md"), "# Invalid\n").unwrap();
    fs::write(target.join("wiki/log.md"), "# Wiki Log\n\n## bad\n").unwrap();

    let output = llmwiki()
        .args(["check", target.to_str().unwrap(), "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    let codes = diagnostic_codes(&output);
    for code in [
        "unregistered-raw",
        "missing-raw",
        "invalid-manifest-entry",
        "missing-frontmatter",
        "invalid-filename",
        "index-unreachable",
        "orphan",
        "invalid-log-heading",
    ] {
        assert!(codes.contains(&code), "missing diagnostic: {code}");
    }
}

#[test]
fn check_reports_missing_required_structure() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    fs::remove_file(target.join("wiki/log.md")).unwrap();
    fs::remove_file(target.join("raw/.gitkeep")).unwrap();
    fs::remove_dir(target.join("raw")).unwrap();
    let output = llmwiki()
        .args(["check", target.to_str().unwrap(), "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    let paths = output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            item["code"]
                .as_str()
                .unwrap()
                .starts_with("missing-required")
        })
        .map(|item| item["path"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"raw"));
    assert!(paths.contains(&"wiki/log.md"));
}

#[test]
fn check_detects_git_log_rewrites_but_allows_appends() {
    let temporary = tempdir().expect("temporary directory");
    let target = temporary.path().join("wiki");
    llmwiki()
        .args(["init", target.to_str().unwrap()])
        .assert()
        .success();
    git(&target, &["init"]);
    git(&target, &["add", "."]);
    git(
        &target,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-m",
            "initial",
        ],
    );
    let log = target.join("wiki/log.md");
    fs::write(
        &log,
        format!(
            "{}\n## [2026-08-14] ingest | Added evidence\n",
            fs::read_to_string(&log).unwrap()
        ),
    )
    .unwrap();
    llmwiki()
        .args(["check", target.to_str().unwrap()])
        .assert()
        .success();
    fs::write(&log, "# Wiki Log\n## [2026-08-14] ingest | Rewritten\n").unwrap();
    let output = llmwiki()
        .args(["check", target.to_str().unwrap(), "--format", "json"])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["code"] == "log-history-modified"));
}

fn write_page(
    target: &std::path::Path,
    relative: &str,
    title: &str,
    page_type: &str,
    body: &str,
    extra: &str,
) {
    fs::write(
        target.join("wiki").join(format!("{relative}.md")),
        format!("---\ntitle: {title}\ntype: {page_type}\ncreated: \"2026-08-14\"\nupdated: \"2026-08-14\"\n{extra}---\n\n{body}\n"),
    )
    .unwrap();
}

fn git(target: &std::path::Path, args: &[&str]) {
    ProcessCommand::new("git")
        .arg("-C")
        .arg(target)
        .args(args)
        .output()
        .unwrap();
}

fn diagnostic_codes(output: &Value) -> Vec<&str> {
    output["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["code"].as_str().unwrap())
        .collect()
}
