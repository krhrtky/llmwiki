use clap::{error::ErrorKind, Parser, Subcommand, ValueEnum};
use llmwiki::commands::{run_lint_command, unsupported_command};
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
        #[arg(long, value_enum)]
        scope: Option<ScopeArg>,
        paths: Vec<PathBuf>,
    },
    Query {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long)]
        question: Option<String>,
        #[arg(long, value_enum)]
        scope: Option<ScopeArg>,
    },
    File {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, value_enum)]
        scope: Option<ScopeArg>,
        #[arg(long)]
        owner: Option<String>,
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
        #[arg(long, value_enum)]
        from_scope: Option<ScopeArg>,
        #[arg(long, value_enum)]
        to_scope: Option<ScopeArg>,
        #[arg(long)]
        reviewer: Option<String>,
        #[arg(long)]
        approver: Option<String>,
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
        Command::Ingest { .. } => unsupported_command("ingest"),
        Command::Query { .. } => unsupported_command("query"),
        Command::File { .. } => unsupported_command("file"),
        Command::Graph { .. } => unsupported_command("graph"),
        Command::Propose { .. } => unsupported_command("propose"),
        Command::Redact { .. } => unsupported_command("redact"),
        Command::Export { .. } => unsupported_command("export"),
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
