mod cli;
mod completion;
mod dates;
mod models;
mod output;
mod parser;
mod queries;
mod validation;
mod write;

use crate::cli::{Cli, Commands};
use crate::models::{Account, Accounts, Journal, JournalMetadata, Posting, Transaction};
use crate::parser::parse_journal;
use crate::validation::{
    validate_account_path, validate_journal, validate_metadata, validate_transaction,
};
use clap::{CommandFactory, Parser};
use jiff::Zoned;
use std::fs;

fn main() {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();
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
    // Initialization must work before a journal exists.
    if let Commands::Init {
        name,
        year,
        currency,
    } = cli.command
    {
        let metadata = JournalMetadata {
            name: name.trim().to_string(),
            year,
            currency,
        };
        validate_metadata(&metadata)?;
        let journal = Journal {
            metadata,
            accounts: Accounts::new(),
            transactions: Vec::new(),
        };
        write::create_journal(cli.journal_path.clone(), &journal).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("{} already exists", cli.journal_path)
            } else {
                format!("failed creating {}: {error}", cli.journal_path)
            }
        })?;
        return Ok(Some(format!("Created {}.\n", cli.journal_path)));
    }

    let mut journal = load_journal(&cli.journal_path)?;

    match cli.command {
        Commands::Init { .. } => unreachable!("initialization handled before loading"),
        Commands::Sort => {
            journal.sort_transactions();
            write::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(Some("Journal sorted.\n".to_string()))
        }
        Commands::List => Ok(Some(write::format_accounts(&journal.accounts))),
        Commands::Add {
            account_type,
            account,
        } => {
            let path = format!("{}/{}", account_type.root_name(), account);
            validate_account_path(&path).map_err(|error| format!("invalid account: {error}"))?;
            let (parent_path, name) = path
                .rsplit_once('/')
                .expect("root prefix includes separator");
            let parent = journal
                .accounts
                .find_account_mut(parent_path)
                .ok_or_else(|| format!("unknown parent account `{parent_path}`"))?;
            if parent.children.iter().any(|child| child.name == name) {
                return Err(format!("account `{path}` already exists"));
            }
            parent.children.push(Account {
                name: name.to_string(),
                children: Vec::new(),
            });
            write::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(None)
        }
        Commands::Check => Ok(Some(format!("{} is valid\n", cli.journal_path))),
        Commands::Pnl {
            period,
            shortcuts,
            depth,
        } => {
            let range = period
                .request(shortcuts.selection())
                .resolve(journal.metadata.year, Zoned::now().date())?;
            Ok(Some(output::format_pnl(
                &queries::pnl(&journal, range),
                journal.metadata.year,
                depth,
            )))
        }
        Commands::Bs { date, depth } => {
            let range = date
                .request()
                .resolve(journal.metadata.year, Zoned::now().date())?;
            Ok(Some(output::format_bs(
                &queries::bs(&journal, range),
                journal.metadata.year,
                depth,
            )))
        }
        Commands::Ledger {
            account,
            period,
            shortcuts,
        } => {
            let range = period
                .request(shortcuts.selection())
                .resolve(journal.metadata.year, Zoned::now().date())?;
            let report = queries::ledger(&journal, &account, range)
                .ok_or_else(|| format!("unknown account `{account}`"))?;
            Ok(Some(output::format_ledger(
                &account,
                &report,
                journal.metadata.year,
            )))
        }
        Commands::Post {
            amount,
            from_account,
            to_account,
            description,
            date,
        } => {
            let date = dates::parse_date(journal.metadata.year, &date)?;
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

            let transaction = Transaction::new(date, description.join(" "), postings);
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
