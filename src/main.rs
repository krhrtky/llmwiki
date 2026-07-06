use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use llmwiki::commands::{
    run_codex_session_import_command, run_export_command, run_export_command_with_store,
    run_file_command, run_graph_command, run_ingest_command, run_lint_command, run_propose_command,
    run_propose_command_with_stores, run_query_command, run_query_command_with_store,
    run_redact_command, run_related_command, run_related_command_with_store,
    run_skill_install_command,
};
use llmwiki::storage::{resolve_store, StoreContext};
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
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        scope: Option<String>,
        paths: Vec<PathBuf>,
    },
    Query {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
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
        #[arg(long = "retrieval-scope")]
        retrieval_scope: Vec<PathBuf>,
    },
    Related {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
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
        #[arg(long = "retrieval-scope")]
        retrieval_scope: Vec<PathBuf>,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long)]
        limit: Option<usize>,
        seed: Option<PathBuf>,
    },
    File {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
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
        candidate: Option<PathBuf>,
    },
    Graph {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        paths: Vec<PathBuf>,
    },
    Propose {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long = "from-store")]
        from_store: Option<String>,
        #[arg(long = "to-store")]
        to_store: Option<String>,
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
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, value_enum)]
        target_scope: Option<ScopeArg>,
        paths: Vec<PathBuf>,
    },
    Export {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
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
        #[arg(long = "export-scope")]
        export_scope: Vec<PathBuf>,
        paths: Vec<PathBuf>,
    },
    Lint {
        #[arg(long)]
        workspace_root: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        store: Option<String>,
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
    CodexSession {
        #[command(subcommand)]
        command: CodexSessionCommand,
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

#[derive(Debug, Subcommand)]
enum CodexSessionCommand {
    Import {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long = "sessions-root")]
        sessions_root: Option<PathBuf>,
        #[arg(long = "repo-root")]
        repo_root: Option<PathBuf>,
        #[arg(long)]
        limit: Option<usize>,
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
            config,
            store,
            paths,
            ..
        } => match resolve_store_or_workspace("lint", config, store, workspace_root) {
            Ok(resolution) => run_lint_command(resolution.workspace_root(), &paths),
            Err(value) => Ok(value),
        },
        Command::Skill { command } => match command {
            SkillCommand::Install {
                workspace_root,
                codex_home,
            } => run_skill_install_command(&workspace_root, codex_home),
        },
        Command::CodexSession { command } => match command {
            CodexSessionCommand::Import {
                workspace_root,
                sessions_root,
                repo_root,
                limit,
                ..
            } => run_codex_session_import_command(&workspace_root, sessions_root, repo_root, limit),
        },
        Command::Ingest {
            workspace_root,
            config,
            store,
            paths,
            scope,
            ..
        } => {
            let (workspace_root, scope) =
                match resolve_workspace_and_scope("ingest", config, store, workspace_root, scope) {
                    Ok(value) => value,
                    Err(value) => print_and_exit(value),
                };
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
            config,
            store,
            question,
            scope,
            content_level,
            subject_kind,
            subject_id,
            retrieval_scope,
            ..
        } => {
            let value = match resolve_store_or_workspace("query", config, store, workspace_root) {
                Ok(StoreResolution::Store(store_context)) => run_query_command_with_store(
                    store_context,
                    question,
                    scope,
                    content_level,
                    subject_kind,
                    subject_id,
                    retrieval_scope,
                ),
                Ok(StoreResolution::Workspace(workspace_root)) => run_query_command(
                    &workspace_root,
                    question,
                    scope,
                    content_level,
                    subject_kind,
                    subject_id,
                    retrieval_scope,
                ),
                Err(value) => Ok(value),
            };
            let value = match value {
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
                        "scope_evaluations": [],
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
            config,
            store,
            seed,
            operation,
            scope,
            content_level,
            subject_kind,
            subject_id,
            retrieval_scope,
            depth,
            limit,
            ..
        } => match resolve_store_or_workspace("related", config, store, workspace_root) {
            Ok(StoreResolution::Store(store_context)) => run_related_command_with_store(
                store_context,
                seed,
                operation,
                scope,
                content_level,
                subject_kind,
                subject_id,
                retrieval_scope,
                depth,
                limit,
            ),
            Ok(StoreResolution::Workspace(workspace_root)) => run_related_command(
                &workspace_root,
                seed,
                operation,
                scope,
                content_level,
                subject_kind,
                subject_id,
                retrieval_scope,
                depth,
                limit,
            ),
            Err(value) => Ok(value),
        },
        Command::File {
            workspace_root,
            config,
            store,
            scope,
            owner,
            reviewer,
            risk_owner,
            confidence,
            citation,
            candidate,
            ..
        } => {
            let (workspace_root, scope) =
                match resolve_workspace_and_scope("file", config, store, workspace_root, scope) {
                    Ok(value) => value,
                    Err(value) => print_and_exit(value),
                };
            run_file_command(
                &workspace_root,
                candidate,
                scope,
                owner,
                reviewer,
                risk_owner,
                confidence,
                citation,
            )
        }
        Command::Graph {
            workspace_root,
            config,
            store,
            paths,
            ..
        } => match resolve_store_or_workspace("graph", config, store, workspace_root) {
            Ok(resolution) => run_graph_command(resolution.workspace_root(), &paths),
            Err(value) => Ok(value),
        },
        Command::Propose {
            workspace_root,
            config,
            from_store,
            to_store,
            paths,
            from_scope,
            to_scope,
            reviewer,
            approver,
            redaction_report,
            ..
        } => {
            let legacy_workspace_root =
                workspace_root.clone().unwrap_or_else(|| PathBuf::from("."));
            match resolve_propose_stores("propose", config, from_store, to_store, workspace_root) {
                Ok(Some((from_store, to_store))) => run_propose_command_with_stores(
                    from_store,
                    to_store,
                    &paths,
                    reviewer,
                    approver,
                    redaction_report,
                ),
                Ok(None) => run_propose_command(
                    &legacy_workspace_root,
                    &paths,
                    from_scope,
                    to_scope,
                    reviewer,
                    approver,
                    redaction_report,
                ),
                Err(value) => Ok(value),
            }
        }
        Command::Redact {
            workspace_root,
            config,
            store,
            target_scope,
            paths,
            ..
        } => match resolve_store_or_workspace("redact", config, store, workspace_root) {
            Ok(StoreResolution::Store(store_context)) => run_redact_command(
                &store_context.canonical_root,
                target_scope.map(scope_arg_to_string),
                &paths,
            ),
            Ok(StoreResolution::Workspace(workspace_root)) => run_redact_command(
                &workspace_root,
                target_scope.map(scope_arg_to_string),
                &paths,
            ),
            Err(value) => Ok(value),
        },
        Command::Export {
            workspace_root,
            config,
            store,
            scope,
            content_level,
            subject_kind,
            subject_id,
            export_scope,
            paths,
            ..
        } => {
            let value = match resolve_store_or_workspace("export", config, store, workspace_root) {
                Ok(StoreResolution::Store(store_context)) => run_export_command_with_store(
                    store_context,
                    &paths,
                    scope.map(scope_arg_to_string),
                    content_level,
                    subject_kind,
                    subject_id,
                    export_scope,
                ),
                Ok(StoreResolution::Workspace(workspace_root)) => run_export_command(
                    &workspace_root,
                    &paths,
                    scope.map(scope_arg_to_string),
                    content_level,
                    subject_kind,
                    subject_id,
                    export_scope,
                ),
                Err(value) => Ok(value),
            };
            let value = match value {
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

enum StoreResolution {
    Workspace(PathBuf),
    Store(StoreContext),
}

impl StoreResolution {
    fn workspace_root(&self) -> &PathBuf {
        match self {
            Self::Workspace(path) => path,
            Self::Store(context) => &context.canonical_root,
        }
    }
}

fn resolve_store_or_workspace(
    command: &str,
    config: Option<PathBuf>,
    store: Option<String>,
    workspace_root: Option<PathBuf>,
) -> Result<StoreResolution, serde_json::Value> {
    match store {
        Some(store) => {
            if workspace_root.is_some() {
                return Err(command_status(
                    command,
                    "error",
                    "--store and --workspace-root cannot be specified together",
                ));
            }
            let config = config.unwrap_or_else(|| PathBuf::from("llmwiki.yaml"));
            resolve_store(&config, &store)
                .map(StoreResolution::Store)
                .map_err(|error| command_status(command, "error", error.to_string()))
        }
        None => Ok(StoreResolution::Workspace(
            workspace_root.unwrap_or_else(|| PathBuf::from(".")),
        )),
    }
}

fn resolve_workspace_and_scope(
    command: &str,
    config: Option<PathBuf>,
    store: Option<String>,
    workspace_root: Option<PathBuf>,
    scope: Option<String>,
) -> Result<(PathBuf, Option<String>), serde_json::Value> {
    match resolve_store_or_workspace(command, config, store, workspace_root)? {
        StoreResolution::Workspace(workspace_root) => Ok((workspace_root, scope)),
        StoreResolution::Store(store_context) => {
            let scope = scope.or_else(|| Some(store_context.legacy_scope()));
            Ok((store_context.canonical_root, scope))
        }
    }
}

fn resolve_propose_stores(
    command: &str,
    config: Option<PathBuf>,
    from_store: Option<String>,
    to_store: Option<String>,
    workspace_root: Option<PathBuf>,
) -> Result<Option<(StoreContext, StoreContext)>, serde_json::Value> {
    if from_store.is_none() && to_store.is_none() {
        return Ok(None);
    }
    if workspace_root.is_some() {
        return Err(command_status(
            command,
            "error",
            "--from-store/--to-store and --workspace-root cannot be specified together",
        ));
    }
    let Some(from_store) = from_store else {
        return Err(command_status(command, "hold", "from_store is required"));
    };
    let Some(to_store) = to_store else {
        return Err(command_status(command, "hold", "to_store is required"));
    };
    let config = config.unwrap_or_else(|| PathBuf::from("llmwiki.yaml"));
    let from_store = resolve_store(&config, &from_store)
        .map_err(|error| command_status(command, "error", error.to_string()))?;
    let to_store = resolve_store(&config, &to_store)
        .map_err(|error| command_status(command, "error", error.to_string()))?;
    Ok(Some((from_store, to_store)))
}

fn command_status(command: &str, status: &str, message: impl ToString) -> serde_json::Value {
    serde_json::json!({
        "command_result": {
            "command": command,
            "status": status,
            "message": message.to_string()
        }
    })
}

fn print_and_exit(value: serde_json::Value) -> ! {
    print_json(&value);
    std::process::exit(1);
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
retrieval_scope:
  rule_id: query-allow
  subject:
    kind: user
    id: alice
  scope: team
  operation: query
  content_level: content
  resource:
    type: concept_document
    selector: "*"
  selection: include
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
            "--retrieval-scope",
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
                retrieval_scope,
                ..
            } => run_query_command(
                &workspace_root.unwrap_or_else(|| PathBuf::from(".")),
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                retrieval_scope,
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
            "--retrieval-scope",
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
                retrieval_scope,
                ..
            } => run_query_command(
                &workspace_root.unwrap_or_else(|| PathBuf::from(".")),
                question,
                scope,
                content_level,
                subject_kind,
                subject_id,
                retrieval_scope,
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
                let workspace_root = workspace_root.unwrap_or_else(|| PathBuf::from("."));
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
                let workspace_root = workspace_root.unwrap_or_else(|| PathBuf::from("."));
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
            let workspace_root = workspace_root.unwrap_or_else(|| PathBuf::from("."));
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
            let workspace_root = workspace_root.unwrap_or_else(|| PathBuf::from("."));
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
                let workspace_root = workspace_root.unwrap_or_else(|| PathBuf::from("."));
                let value = run_ingest_command(&workspace_root, &paths, scope).unwrap();
                assert_eq!(value["ingest_result"]["status"], "hold");
            }
            _ => panic!("expected ingest command"),
        }
    }
}
