mod operations;
mod validation;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "llmwiki",
    about = "Personal LLM Wiki initializer and structural verifier"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create an empty personal Wiki.
    Init { target: PathBuf },
    /// Manage immutable raw sources.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// Search Wiki pages and textual raw sources.
    Search { target: PathBuf, query: String },
    /// Validate Wiki structure.
    Check {
        target: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Debug, Subcommand)]
enum SourceCommand {
    /// Register local source files.
    Add {
        target: PathBuf,
        #[arg(num_args = 1..)]
        files: Vec<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };

    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("llmwiki: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<u8, operations::LlmwikiError> {
    match cli.command {
        Command::Init { target } => {
            operations::initialize(&target)?;
            println!("Initialized personal Wiki: {}", target.display());
            Ok(0)
        }
        Command::Source {
            command: SourceCommand::Add { target, files },
        } => {
            for message in operations::add_sources(&target, &files)? {
                println!("{message}");
            }
            Ok(0)
        }
        Command::Search { target, query } => {
            for result in operations::search(&target, &query)? {
                println!("{result}");
            }
            Ok(0)
        }
        Command::Check { target, format } => {
            let result = validation::check(&target)?;
            match format {
                OutputFormat::Text => print!("{}", result.as_text()),
                OutputFormat::Json => println!("{}", result.as_json_string()?),
            }
            Ok(if result.errors() == 0 { 0 } else { 1 })
        }
    }
}
