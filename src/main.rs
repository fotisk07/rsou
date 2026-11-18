mod models;
mod output;
mod parser;
mod queries;
mod validation;
mod write;

use crate::models::{Journal, Posting, Transaction};
use crate::parser::parse_journal;
use crate::validation::{ValidationError, validate_journal, validate_transaction};
use clap::{Parser, Subcommand};
use jiff::Zoned;
use jiff::civil::Date;
use rust_decimal::Decimal;
use std::fs;

#[derive(Parser)]
#[command(name = "rsou")]
#[command(about = "Personal accounting CLI")]
struct Cli {
    #[arg(short = 'j', long, global = true, default_value = "journal.sou")]
    journal_path: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Sort,
    Check,
    Pnl,
    Ledger {
        account: String,
    },
    Post {
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

    let mut journal = match load_journal(&cli.journal_path) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Sort => {
            journal.sort_transactions();
            if let Err(error) = write::write_journal(cli.journal_path, &journal) {
                eprintln!("failed saving: {error:?}");
                std::process::exit(1);
            }
        }
        Commands::Check => {
            println!("{} is valid", cli.journal_path);
        }

        Commands::Pnl => {
            print!("{}", output::format_pnl(&queries::pnl(&journal)));
        }
        Commands::Ledger { account } => match queries::ledger_entries(&journal, &account) {
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
                    eprintln!("invalid transaction: {error:?}");
                    std::process::exit(1);
                }
            }

            journal.transactions.push(transaction);
            match write::write_journal(cli.journal_path, &journal) {
                Ok(()) => {}
                Err(error) => {
                    eprintln!("failed saving: {error:?}");
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
        ValidationError::InvalidDescription => {
            "validation error: description must be nonempty and contain no line breaks".to_string()
        }
        ValidationError::TooManyDecimalPlaces(account) => {
            format!(
                "validation error: amount for `{account}` requires more than two decimal places"
            )
        }
    })?;

    Ok(journal)
}
