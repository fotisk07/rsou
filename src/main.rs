mod cli;
mod completion;
mod dates;
mod journal;
mod models;
mod output;
mod queries;
mod recurring;
mod validation;

use crate::cli::{Cli, Commands, RecurCommands};
use crate::models::{
    Account, Accounts, Journal, JournalMetadata, Posting, RecurringTransaction, Transaction,
};
use crate::validation::{
    validate_account_path, validate_metadata, validate_recurring, validate_transaction,
};
use clap::{CommandFactory, Parser};
use jiff::Zoned;
use rust_decimal::Decimal;

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
        author,
        year,
        currency,
    } = cli.command
    {
        let metadata = JournalMetadata {
            name: name.trim().to_string(),
            author: author
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            year,
            currency,
        };
        validate_metadata(&metadata)?;
        let journal = Journal {
            metadata,
            accounts: Accounts::new(),
            aliases: Vec::new(),
            transactions: Vec::new(),
            recurring_transactions: Vec::new(),
        };
        journal::create_journal(cli.journal_path.clone(), &journal).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("{} already exists", cli.journal_path)
            } else {
                format!("failed creating {}: {error}", cli.journal_path)
            }
        })?;
        return Ok(Some(format!("Created {}.\n", cli.journal_path)));
    }

    let mut journal = journal::load_journal(&cli.journal_path)?;

    match cli.command {
        Commands::Init { .. } => unreachable!("initialization handled before loading"),
        Commands::Sort => {
            journal.sort_transactions();
            journal::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(Some("Journal sorted.\n".to_string()))
        }
        Commands::List => Ok(Some(output::format_accounts(&journal.accounts))),
        Commands::Add { account: path } => {
            validate_account_path(&path).map_err(|error| format!("invalid account: {error}"))?;
            let (parent_path, name) = path
                .rsplit_once('/')
                .ok_or_else(|| "specify a full account path, e.g. Assets/Bank".to_string())?;
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
            journal::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(None)
        }
        Commands::Check => Ok(Some(format!("{} is valid\n", cli.journal_path))),
        Commands::Pnl {
            period,
            shortcuts,
            by_month,
            by_quarter,
            depth,
            format,
        } => {
            let range = period
                .request(shortcuts.selection())
                .resolve(journal.metadata.year, Zoned::now().date())?;
            let breakdown = if by_month {
                Some(dates::PeriodBreakdown::Month)
            } else if by_quarter {
                Some(dates::PeriodBreakdown::Quarter)
            } else {
                None
            };
            let columns = if let Some(breakdown) = breakdown {
                queries::pnl_breakdown(&journal, range, breakdown)?
            } else {
                vec![("Amount".to_string(), queries::pnl(&journal, range))]
            };
            output::pnl(&columns, &journal.metadata, depth, format)
        }
        Commands::Bs {
            date,
            depth,
            format,
            by_month,
            by_quarter,
        } => {
            let request = date.request(None);
            let implicit_start = request.shortcut.is_none();
            let mut range = request.resolve(journal.metadata.year, Zoned::now().date())?;
            // The default month-end cutoff should compare from January;
            // only an explicit month/quarter selection narrows the first column.
            if implicit_start && (by_month || by_quarter) {
                range.from = None;
            }
            let breakdown = if by_month {
                Some(dates::PeriodBreakdown::Month)
            } else if by_quarter {
                Some(dates::PeriodBreakdown::Quarter)
            } else {
                None
            };
            let columns = if let Some(breakdown) = breakdown {
                queries::bs_breakdown(&journal, range, breakdown)?
            } else {
                vec![("Amount".to_string(), queries::bs(&journal, range))]
            };
            output::bs(&columns, &journal.metadata, depth, format)
        }
        Commands::Ledger {
            account,
            period,
            shortcuts,
        } => {
            let range = period
                .request(shortcuts.selection())
                .resolve(journal.metadata.year, Zoned::now().date())?;
            let account = journal.resolve_account(&account);
            let report = queries::ledger(&journal, account, range)
                .ok_or_else(|| format!("unknown account `{account}`"))?;
            Ok(Some(output::format_ledger(
                account,
                &report,
                journal.metadata.year,
            )))
        }
        Commands::Recur {
            command: RecurCommands::List { details },
        } => Ok(Some(output::format_recurring(
            &journal.recurring_transactions,
            details,
        ))),
        Commands::Recur {
            command: RecurCommands::Post { cutoff, dry_run },
        } => {
            let through = cutoff
                .request(Some(dates::PeriodShortcut::Today))
                .resolve(journal.metadata.year, Zoned::now().date())?
                .to
                .ok_or_else(|| "missing posting cutoff".to_string())?;
            let first = journal.transactions.len();
            recurring::post_due_transactions(&mut journal, through)?;
            let transactions = &journal.transactions[first..];
            if transactions.is_empty() {
                return Ok(Some("No recurring transactions due.\n".to_string()));
            }
            let action = if dry_run { "Would post" } else { "Posted" };
            let count = transactions.len();
            let noun = if count == 1 {
                "transaction"
            } else {
                "transactions"
            };
            let mut text = format!("{action} {count} recurring {noun}:\n");
            for transaction in transactions {
                text.push_str(&format!(
                    "  {} {}\n",
                    transaction.date, transaction.description
                ));
            }
            if !dry_run {
                journal::write_journal(cli.journal_path, &journal)
                    .map_err(|error| format!("failed saving: {error}"))?;
            }
            Ok(Some(text))
        }
        Commands::Recur {
            command:
                RecurCommands::Add {
                    description,
                    postings,
                    start,
                    frequency,
                },
        } => {
            let start_date = dates::resolve_start_date(
                journal.metadata.year,
                start.as_deref(),
                Zoned::now().date(),
            )?;
            let recurring = RecurringTransaction {
                frequency: frequency.frequency(),
                start_date,
                next_date: start_date,
                description,
                postings: parse_split_postings(postings)?,
            };
            validate_recurring(&journal, &recurring)
                .map_err(|error| format!("invalid recurring template: {error}"))?;
            journal.recurring_transactions.push(recurring);
            journal::write_journal(cli.journal_path, &journal)
                .map_err(|error| format!("failed saving: {error}"))?;
            Ok(None)
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
                    account: journal.resolve_account(&from_account).to_string(),
                    amount: -amount,
                },
                Posting {
                    account: journal.resolve_account(&to_account).to_string(),
                    amount,
                },
            ];

            let transaction = Transaction::new(date, description.join(" "), postings);
            save_transaction(cli.journal_path, journal, transaction)
        }
        Commands::Split {
            description,
            postings,
            date,
        } => {
            let date = dates::parse_date(journal.metadata.year, &date)?;
            let postings = parse_split_postings(postings)?;
            let transaction = Transaction::new(date, description, postings);
            save_transaction(cli.journal_path, journal, transaction)
        }
    }
}

fn parse_split_postings(values: Vec<String>) -> Result<Vec<Posting>, String> {
    if !values.len().is_multiple_of(2) {
        return Err("each account must be followed by an amount".to_string());
    }
    let mut values = values.into_iter();
    std::iter::from_fn(|| Some((values.next()?, values.next()?)))
        .map(|(account, amount)| {
            let amount = amount
                .parse::<Decimal>()
                .map_err(|_| format!("invalid amount `{amount}` for account `{account}`"))?;
            Ok(Posting { account, amount })
        })
        .collect()
}

fn save_transaction(
    path: String,
    mut journal: Journal,
    transaction: Transaction,
) -> Result<Option<String>, String> {
    validate_transaction(&journal, &transaction)
        .map_err(|error| format!("invalid transaction: {error}"))?;
    journal.transactions.push(transaction);
    journal::write_journal(path, &journal).map_err(|error| format!("failed saving: {error}"))?;
    Ok(None)
}
