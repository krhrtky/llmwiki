use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use llmwiki::commands::{
    run_export_command, run_file_command, run_graph_command, run_ingest_command, run_lint_command,
    run_propose_command, run_query_command, run_redact_command, run_related_command,
    run_skill_install_command,
};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "llmwiki")]
#[command(about = "LLMWiki file-first CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Ingest {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        scope: Option<String>,
        paths: Vec<PathBuf>,
    },
    Query {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        question: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long = "content-level")]
        content_level: Option<String>,
        #[arg(long = "subject-kind")]
        subject_kind: Option<String>,
        #[arg(long = "subject-id")]
        subject_id: Option<String>,
        #[arg(long = "access-policy")]
        access_policy: Vec<PathBuf>,
    },
    Related {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        operation: Option<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long = "content-level")]
        content_level: Option<String>,
        #[arg(long = "subject-kind")]
        subject_kind: Option<String>,
        #[arg(long = "subject-id")]
        subject_id: Option<String>,
        #[arg(long = "access-policy")]
        access_policy: Vec<PathBuf>,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long)]
        limit: Option<usize>,
        seed: Option<PathBuf>,
    },
    File {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        reviewer: Option<String>,
        #[arg(long)]
        risk_owner: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long)]
        citation: Vec<String>,
        #[arg(long)]
        access_policy_ref: Vec<String>,
        #[arg(long)]
        candidate: Option<PathBuf>,
    },
    Graph {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        paths: Vec<PathBuf>,
    },
    Propose {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        from_scope: Option<String>,
        #[arg(long)]
        to_scope: Option<String>,
        #[arg(long)]
        reviewer: Option<String>,
        #[arg(long)]
        approver: Option<String>,
        #[arg(long)]
        redaction_report: Option<PathBuf>,
        paths: Vec<PathBuf>,
    },
    Redact {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, value_enum)]
        target_scope: Option<ScopeArg>,
        paths: Vec<PathBuf>,
    },
    Export {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, value_enum)]
        scope: Option<ScopeArg>,
        #[arg(long = "content-level")]
        content_level: Option<String>,
        #[arg(long = "subject-kind")]
        subject_kind: Option<String>,
        #[arg(long = "subject-id")]
        subject_id: Option<String>,
        #[arg(long = "access-policy")]
        access_policy: Vec<PathBuf>,
        paths: Vec<PathBuf>,
    },
    Lint {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, value_enum)]
        scope: Option<ScopeArg>,
        paths: Vec<PathBuf>,
    },
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SkillCommand {
    Install {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long)]
        codex_home: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ScopeArg {
    Personal,
    Team,
    Org,
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let status = match error.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                | ErrorKind::DisplayVersion => "ok",
                _ => "error",
            };
            print_json(&serde_json::json!({
                "command_result": {
                    "command": "cli",
                    "status": status,
                    "message": error.to_string()
                }
            }));
            if status == "ok" {
                std::process::exit(0);
            }
            std::process::exit(2);
        }
    };

    let result = match cli.command {
        Command::Lint {
            workspace_root,
            paths,
            ..
        } => run_lint_command(&workspace_root, &paths),
        Command::Skill { command } => match command {
            SkillCommand::Install {
                workspace_root,
                codex_home,
            } => run_skill_install_command(&workspace_root, codex_home),
        },
        Command::Ingest {
            workspace_root,
            paths,
            scope,
            ..
        } => {
            let value = match run_ingest_command(&workspace_root, &paths, scope) {
                Ok(value) => value,
                Err(error) => serde_json::json!({
                    "ingest_result": {
                        "status": "error",
                        "message": error.to_string(),
                        "generated_at": chrono::Utc::now().to_rfc3339(),
                        "scope": "",
                        "source_paths": [],
                        "artifact_path": "",
                        "manifest_path": "",
                        "candidates": [],
                        "evidence_map": [],
                        "diff_summary": "ingest failed"
                    }
                }),
            };
            Ok(value)
        }
        Command::Query {
            workspace_root,
            question,
            scope,
            content_level,
            subject_kind,
            subject_id,
            access_policy,
            ..
        } => {
            let value = match run_query_command(
                &workspace_root,
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                access_policy,
            ) {
                Ok(value) => value,
                Err(error) => serde_json::json!({
                    "query_result": {
                        "generated_at": chrono::Utc::now().to_rfc3339(),
                        "status": "error",
                        "message": error.to_string(),
                        "question": null,
                        "scope": null,
                        "content_level": null,
                        "answer": "",
                        "citations": [],
                        "confidence": "low",
                        "matched_pages": [],
                        "decision_logs": [],
                        "filing_candidate_metadata": {
                            "source": "query",
                            "scope": "",
                            "content_level": "",
                            "confidence": "low",
                            "citations": [],
                            "owner": null,
                            "reviewer": null,
                            "risk_owner": null,
                            "lifecycle": "draft",
                            "access_policy_refs": [],
                            "subject_kind": null,
                            "subject_id": null
                        }
                    }
                }),
            };
            Ok(value)
        }
        Command::Related {
            workspace_root,
            seed,
            operation,
            scope,
            content_level,
            subject_kind,
            subject_id,
            access_policy,
            depth,
            limit,
            ..
        } => run_related_command(
            &workspace_root,
            seed,
            operation,
            scope,
            content_level,
            subject_kind,
            subject_id,
            access_policy,
            depth,
            limit,
        ),
        Command::File {
            workspace_root,
            scope,
            owner,
            reviewer,
            risk_owner,
            confidence,
            citation,
            access_policy_ref,
            candidate,
            ..
        } => run_file_command(
            &workspace_root,
            candidate,
            scope,
            owner,
            reviewer,
            risk_owner,
            confidence,
            citation,
            access_policy_ref,
        ),
        Command::Graph {
            workspace_root,
            paths,
            ..
        } => run_graph_command(&workspace_root, &paths),
        Command::Propose {
            workspace_root,
            paths,
            from_scope,
            to_scope,
            reviewer,
            approver,
            redaction_report,
            ..
        } => run_propose_command(
            &workspace_root,
            &paths,
            from_scope,
            to_scope,
            reviewer,
            approver,
            redaction_report,
        ),
        Command::Redact {
            workspace_root,
            target_scope,
            paths,
            ..
        } => run_redact_command(
            &workspace_root,
            target_scope.map(scope_arg_to_string),
            &paths,
        ),
        Command::Export {
            workspace_root,
            scope,
            content_level,
            subject_kind,
            subject_id,
            access_policy,
            paths,
            ..
        } => {
            let value = match run_export_command(
                &workspace_root,
                &paths,
                scope.map(scope_arg_to_string),
                content_level,
                subject_kind,
                subject_id,
                access_policy,
            ) {
                Ok(value) => value,
                Err(error) => serde_json::json!({
                    "command_result": {
                        "command": "export",
                        "status": "error",
                        "message": error.to_string()
                    }
                }),
            };
            Ok(value)
        }
    };

    match result {
        Ok(value) => print_json(&value),
        Err(error) => {
            print_json(&serde_json::json!({
                "command_result": {
                    "command": "cli",
                    "status": "error",
                    "message": error.to_string()
                }
            }));
            std::process::exit(1);
        }
    }
}

fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("JSON serialization must succeed")
    );
}

#[cfg(test)]
mod query_cli_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn query_cli_smoke_success_and_invalid_scope_hold() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        write_file(
            dir.path().join("docs").join("index.md"),
            "---\nllmwiki:\n  scope: team\n---\n# Index\nDeterministic query page.\n",
        );
        write_file(
            dir.path().join("policy.yaml"),
            r#"
policy:
  policy_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  decision: allow
  reason: allow query
"#,
        );

        let cli = Cli::try_parse_from([
            "llmwiki",
            "query",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "--question",
            "deterministic query",
            "--scope",
            "team",
            "--content-level",
            "content",
            "--subject-kind",
            "user",
            "--subject-id",
            "alice",
            "--access-policy",
            "policy.yaml",
        ])
        .unwrap();
        let success = match cli.command {
            Command::Query {
                workspace_root,
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                access_policy,
                ..
            } => run_query_command(
                &workspace_root,
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                access_policy,
            )
            .unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(
            success["query_result"]["status"],
            "success",
            "{}",
            serde_json::to_string_pretty(&success).unwrap()
        );

        let cli = Cli::try_parse_from([
            "llmwiki",
            "query",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "--question",
            "deterministic query",
            "--scope",
            "global",
            "--content-level",
            "content",
            "--subject-kind",
            "user",
            "--subject-id",
            "alice",
            "--access-policy",
            "policy.yaml",
        ])
        .unwrap();
        let invalid_scope = match cli.command {
            Command::Query {
                workspace_root,
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                access_policy,
                ..
            } => run_query_command(
                &workspace_root,
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                access_policy,
            )
            .unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(
            invalid_scope["query_result"]["status"],
            "hold",
            "{}",
            serde_json::to_string_pretty(&invalid_scope).unwrap()
        );
    }

    fn write_file(path: PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

fn scope_arg_to_string(scope: ScopeArg) -> String {
    match scope {
        ScopeArg::Personal => "personal".to_string(),
        ScopeArg::Team => "team".to_string(),
        ScopeArg::Org => "org".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llmwiki::commands::run_ingest_command;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn propose_accepts_unknown_scope_strings_and_returns_hold() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("index.md"), "# Index\n").unwrap();
        fs::write(dir.path().join("source.md"), "# Source\n").unwrap();
        fs::create_dir_all(dir.path().join(".llmwiki").join("redactions")).unwrap();
        let report_path = dir
            .path()
            .join(".llmwiki")
            .join("redactions")
            .join("source.report.json");
        let report = serde_json::json!({
            "redaction_report": {
                "generated_at": "2026-07-05T00:00:00Z",
                "target_scope": "team",
                "source_paths": ["source.md"],
                "report_path": report_path.to_string_lossy(),
                "draft_path": report_path.to_string_lossy(),
                "recommendation": "allow",
                "findings": [],
                "transformations": [],
                "residual_risk": [],
                "blocked_items": []
            }
        });
        fs::write(
            &report_path,
            format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
        )
        .unwrap();

        let cli = Cli::try_parse_from([
            "llmwiki",
            "propose",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "--from-scope",
            "nonsense",
            "--to-scope",
            "team",
            "--reviewer",
            "alice",
            "--approver",
            "bob",
            "--redaction-report",
            ".llmwiki/redactions/source.report.json",
            "source.md",
        ])
        .unwrap();

        match cli.command {
            Command::Propose {
                workspace_root,
                paths,
                from_scope,
                to_scope,
                reviewer,
                approver,
                redaction_report,
                ..
            } => {
                let value = run_propose_command(
                    &workspace_root,
                    &paths,
                    from_scope,
                    to_scope,
                    reviewer,
                    approver,
                    redaction_report,
                )
                .unwrap();
                assert_eq!(value["command_result"]["status"], "hold");
            }
            _ => panic!("expected propose command"),
        }
    }

    #[test]
    fn ingest_cli_returns_ingest_result_for_txt_source() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("index.md"), "# Index\n").unwrap();
        fs::write(dir.path().join("source.txt"), "raw body\n").unwrap();

        let cli = Cli::try_parse_from([
            "llmwiki",
            "ingest",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "--scope",
            "team",
            "source.txt",
        ])
        .unwrap();

        match cli.command {
            Command::Ingest {
                workspace_root,
                paths,
                scope,
                ..
            } => {
                let value = run_ingest_command(&workspace_root, &paths, scope).unwrap();
                assert!(value.get("ingest_result").is_some());
            }
            _ => panic!("expected ingest command"),
        }
    }

    #[test]
    fn ingest_cli_missing_scope_or_paths_returns_hold() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("index.md"), "# Index\n").unwrap();
        fs::write(dir.path().join("source.txt"), "raw body\n").unwrap();

        let missing_scope = Cli::try_parse_from([
            "llmwiki",
            "ingest",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "source.txt",
        ])
        .unwrap();
        if let Command::Ingest {
            workspace_root,
            paths,
            scope,
            ..
        } = missing_scope.command
        {
            let value = run_ingest_command(&workspace_root, &paths, scope).unwrap();
            assert_eq!(value["ingest_result"]["status"], "hold");
        }

        let missing_paths = Cli::try_parse_from([
            "llmwiki",
            "ingest",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "--scope",
            "team",
        ])
        .unwrap();
        if let Command::Ingest {
            workspace_root,
            paths,
            scope,
            ..
        } = missing_paths.command
        {
            let value = run_ingest_command(&workspace_root, &paths, scope).unwrap();
            assert_eq!(value["ingest_result"]["status"], "hold");
        }
    }

    #[test]
    fn ingest_cli_invalid_scope_is_parsed_and_returns_hold() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("index.md"), "# Index\n").unwrap();
        fs::write(dir.path().join("source.txt"), "raw body\n").unwrap();

        let cli = Cli::try_parse_from([
            "llmwiki",
            "ingest",
            "--workspace-root",
            dir.path().to_str().unwrap(),
            "--scope",
            "nonsense",
            "source.txt",
        ])
        .unwrap();

        match cli.command {
            Command::Ingest {
                workspace_root,
                paths,
                scope,
                ..
            } => {
                let value = run_ingest_command(&workspace_root, &paths, scope).unwrap();
                assert_eq!(value["ingest_result"]["status"], "hold");
            }
            _ => panic!("expected ingest command"),
        }
    }
}
