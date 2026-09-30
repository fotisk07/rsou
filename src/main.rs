mod models;
mod output;
mod parser;
mod queries;
mod validation;
use crate::models::Journal;
use crate::parser::parse_journal;
use crate::validation::{ValidationError, validate_journal};
use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser)]
#[command(name = "rsou")]
#[command(about = "Personal accounting CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Check {
        #[arg(short, long, default_value = "journal.sou")]
        journal_path: String,
    },
    Ledger {
        #[arg(short, long, default_value = "journal.sou")]
        journal_path: String,
        account: String,
    },
}
fn main() {
    let cli = Cli::parse();

    let journal_path = match &cli.command {
        Commands::Check { journal_path } => journal_path,
        Commands::Ledger { journal_path, .. } => journal_path,
    };
    let journal = match load_journal(journal_path) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Check { journal_path } => {
            println!("{journal_path} is valid");
        }

        Commands::Ledger { account, .. } => match queries::ledger_entries(&journal, &account) {
            Some(entries) => {
                print!("{}", output::format_ledger(&account, &entries));
            }
            None => {
                eprintln!("unknown account `{account}`");
                std::process::exit(1);
            }
        },
    }
}

fn load_journal(path: &str) -> Result<Journal, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;

    let journal = parse_journal(&contents).map_err(|error| match error.line {
        Some(line) => format!("parse error (line {line}): {}", error.message),
        None => format!("parse error: {}", error.message),
    })?;

    validate_journal(&journal).map_err(|error| match error {
        ValidationError::UnknownAccount(account) => {
            format!("validation error: unknown account `{account}`")
        }
        ValidationError::Unbalanced(description) => {
            format!("validation error: unbalanced transaction `{description}`")
        }
        ValidationError::NotEnoughPostings => {
            "validation error: transaction needs at least two postings".to_string()
        }
    })?;

    Ok(journal)
}
