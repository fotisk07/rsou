mod completion;
mod dates;
mod models;
mod output;
mod parser;
mod queries;
mod validation;
mod write;

use crate::dates::{DateRange, PeriodShortcut};
use crate::models::{Account, Journal, Posting, Transaction};
use crate::parser::parse_journal;
use crate::validation::{validate_account_path, validate_journal, validate_transaction};
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::ArgValueCompleter;
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
    Pnl {
        #[command(flatten)]
        period: PeriodArgs,
        #[command(flatten)]
        shortcuts: PnlShortcuts,
    },
    Bs {
        /// Include all transactions through DD-MM in the journal year.
        #[arg(long, value_name = "DD-MM")]
        at: Option<String>,
    },
    List,
    Add {
        #[arg(value_enum)]
        account_type: AccountType,
        /// Account path relative to the root, e.g. Bank/Checking.
        account: String,
    },
    Ledger {
        #[arg(add = ArgValueCompleter::new(completion::accounts))]
        account: String,
        #[command(flatten)]
        period: PeriodArgs,
    },
    Post {
        amount: Decimal,
        #[arg(add = ArgValueCompleter::new(completion::accounts))]
        from_account: String,
        #[arg(add = ArgValueCompleter::new(completion::accounts))]
        to_account: String,
        description: String,
        #[arg(short, long, default_value_t = Zoned::now().date())]
        date: Date,
    },
}
#[derive(Args)]
struct PeriodArgs {
    /// Include transactions from DD-MM in the journal year (inclusive).
    #[arg(long, value_name = "DD-MM")]
    from: Option<String>,
    /// Include transactions through DD-MM in the journal year (inclusive).
    #[arg(long, value_name = "DD-MM")]
    to: Option<String>,
}

#[derive(Args)]
#[group(multiple = false, conflicts_with_all = ["from", "to"])]
struct PnlShortcuts {
    /// Report a complete month of the journal year (1-12).
    #[arg(short = 'm', long, value_parser = clap::value_parser!(u8).range(1..=12))]
    month: Option<u8>,
    /// Report a complete quarter of the journal year (1-4).
    #[arg(short = 'q', long, value_parser = clap::value_parser!(u8).range(1..=4))]
    quarter: Option<u8>,
    /// Report from the start of the current month through today.
    #[arg(long)]
    mtd: bool,
    /// Report from the start of the current quarter through today.
    #[arg(long)]
    qtd: bool,
    /// Report from the start of the current year through today.
    #[arg(long)]
    ytd: bool,
    /// Report the previous complete month within the current year.
    #[arg(long)]
    last_month: bool,
}

impl PnlShortcuts {
    fn selection(&self) -> Option<PeriodShortcut> {
        match self {
            Self {
                month: Some(month), ..
            } => Some(PeriodShortcut::Month(*month)),
            Self {
                quarter: Some(quarter),
                ..
            } => Some(PeriodShortcut::Quarter(*quarter)),
            Self { mtd: true, .. } => Some(PeriodShortcut::MonthToDate),
            Self { qtd: true, .. } => Some(PeriodShortcut::QuarterToDate),
            Self { ytd: true, .. } => Some(PeriodShortcut::YearToDate),
            Self {
                last_month: true, ..
            } => Some(PeriodShortcut::LastMonth),
            _ => None,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum AccountType {
    #[value(name = "a")]
    Assets,
    #[value(name = "l")]
    Liabilities,
    #[value(name = "eq")]
    Equity,
    #[value(name = "i")]
    Income,
    #[value(name = "e")]
    Expenses,
}

impl AccountType {
    fn root_name(&self) -> &'static str {
        match self {
            Self::Assets => "Assets",
            Self::Liabilities => "Liabilities",
            Self::Equity => "Equity",
            Self::Income => "Income",
            Self::Expenses => "Expenses",
        }
    }
}

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
    let mut journal = load_journal(&cli.journal_path)?;

    match cli.command {
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
        Commands::Pnl { period, shortcuts } => {
            let range = match shortcuts.selection() {
                Some(shortcut) => shortcut.resolve(journal.metadata.year, Zoned::now().date())?,
                None => DateRange::resolve(
                    journal.metadata.year,
                    period.from.as_deref(),
                    period.to.as_deref(),
                )?,
            };
            Ok(Some(output::format_pnl(
                &queries::pnl(&journal, range),
                journal.metadata.year,
            )))
        }
        Commands::Bs { at } => {
            let range = DateRange::resolve(journal.metadata.year, None, at.as_deref())?;
            Ok(Some(output::format_bs(
                &queries::bs(&journal, range),
                journal.metadata.year,
            )))
        }
        Commands::Ledger { account, period } => {
            let range = DateRange::resolve(
                journal.metadata.year,
                period.from.as_deref(),
                period.to.as_deref(),
            )?;
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
