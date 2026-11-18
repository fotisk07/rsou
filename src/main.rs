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
    Bs,
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
    match run(Cli::parse()) {
        Ok(Some(text)) => print!("{text}"),
        Ok(None) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<Option<String>, String> {
    let mut journal = load_journal(&cli.journal_path)?;

    match cli.command {
        Commands::Sort => {
            journal.sort_transactions();
            write::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(Some("Journal sorted.\n".to_string()))
        }
        Commands::Check => Ok(Some(format!("{} is valid\n", cli.journal_path))),
        Commands::Pnl => Ok(Some(output::format_pnl(&queries::pnl(&journal)))),
        Commands::Bs => Ok(Some(output::format_bs(&queries::bs(&journal)))),
        Commands::Ledger { account } => {
            let entries = queries::ledger_entries(&journal, &account)
                .ok_or_else(|| format!("unknown account `{account}`"))?;
            Ok(Some(output::format_ledger(&account, &entries)))
        }
        Commands::Post {
            amount,
            from_account,
            to_account,
            description,
            date,
        } => {
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
            validate_transaction(&journal.accounts, &transaction)
                .map_err(|error| format!("invalid transaction: {error}"))?;

            journal.transactions.push(transaction);
            write::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(None)
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
