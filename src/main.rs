mod models;
mod output;
mod parser;
mod queries;
mod validation;
mod write;

use crate::models::{Journal, Posting, Transaction};
use crate::parser::parse_journal;
use crate::validation::{validate_journal, validate_transaction};
use clap::{Parser, Subcommand};
use jiff::Zoned;
use jiff::civil::Date;
use rust_decimal::Decimal;
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
    Post {
        #[arg(short, long, default_value = "journal.sou")]
        journal_path: String,
        amount: Decimal,
        from_account: String,
        to_account: String,
        description: String,
        #[arg(short, long, default_value_t = Zoned::now().date())]
        date: Date,
    },
}
fn main() {
    let cli = Cli::parse();

    let journal_path = match &cli.command {
        Commands::Check { journal_path } => journal_path,
        Commands::Ledger { journal_path, .. } => journal_path,
        Commands::Post { journal_path, .. } => journal_path,
    };
    let mut journal = match load_journal(journal_path) {
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
        Commands::Post {
            amount,
            from_account,
            to_account,
            description,
            date,
            journal_path,
        } => {
            if amount <= Decimal::ZERO {
                eprintln!("amount must be greater than zero");
                std::process::exit(1);
            }

            let postings = vec![
                Posting {
                    account: from_account,
                    amount: -amount,
                },
                Posting {
                    account: to_account,
                    amount,
                },
            ];

            let transaction = Transaction::new(date, description, postings);
            match validate_transaction(&journal.accounts, &transaction) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("invalid transaction: {error}");
                    std::process::exit(1);
                }
            }

            journal.transactions.push(transaction);
            journal.sort_transactions();
            match write::write_journal(journal_path, &journal) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("failed saving: {error}");
                    std::process::exit(1);
                }
            };
        }
    }
}

fn load_journal(path: &str) -> Result<Journal, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;

    let journal = parse_journal(&contents).map_err(|error| match error.line {
        Some(line) => format!("parse error (line {line}): {}", error.message),
        None => format!("parse error: {}", error.message),
    })?;

    validate_journal(&journal).map_err(|error| format!("validation error: {error}"))?;

    Ok(journal)
}
