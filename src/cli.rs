use crate::{
    completion,
    dates::{DateRequest, PeriodShortcut},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::ArgValueCompleter;
use jiff::{Zoned, civil::Date};
use rust_decimal::Decimal;

#[derive(Parser)]
#[command(name = "rsou")]
#[command(about = "Personal accounting CLI")]
pub(crate) struct Cli {
    #[arg(short = 'j', long, global = true, default_value = "journal.sou")]
    pub(crate) journal_path: String,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    Sort,
    Check,
    Pnl {
        #[command(flatten)]
        period: PeriodArgs,
        #[command(flatten)]
        shortcuts: PeriodShortcuts,
        /// Number of account levels to display; 0 shows totals only.
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
    Bs {
        /// Include all transactions through DD-MM in the journal year.
        #[arg(long, value_name = "DD-MM")]
        at: Option<String>,
        /// Number of account levels to display; 0 shows totals only.
        #[arg(long, default_value_t = 1)]
        depth: usize,
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
        #[command(flatten)]
        shortcuts: PeriodShortcuts,
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
pub(crate) struct PeriodArgs {
    /// Include transactions from DD-MM in the journal year (inclusive).
    #[arg(long, value_name = "DD-MM")]
    from: Option<String>,
    /// Include transactions through DD-MM in the journal year (inclusive).
    #[arg(long, value_name = "DD-MM")]
    to: Option<String>,
}

impl PeriodArgs {
    pub(crate) fn request(&self, shortcut: Option<PeriodShortcut>) -> DateRequest<'_> {
        DateRequest {
            from: self.from.as_deref(),
            to: self.to.as_deref(),
            shortcut,
        }
    }
}

#[derive(Args)]
#[group(multiple = false, conflicts_with_all = ["from", "to"])]
pub(crate) struct PeriodShortcuts {
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

impl PeriodShortcuts {
    pub(crate) fn selection(&self) -> Option<PeriodShortcut> {
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
pub(crate) enum AccountType {
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
    pub(crate) fn root_name(&self) -> &'static str {
        match self {
            Self::Assets => "Assets",
            Self::Liabilities => "Liabilities",
            Self::Equity => "Equity",
            Self::Income => "Income",
            Self::Expenses => "Expenses",
        }
    }
}
