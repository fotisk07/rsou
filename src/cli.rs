use crate::{
    completion,
    dates::{DateRequest, PeriodShortcut},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::ArgValueCompleter;
use jiff::Zoned;
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
    /// Create a new journal without overwriting an existing file.
    Init {
        #[arg(long, default_value = "Personal journal")]
        name: String,
        #[arg(short = 'y', long, default_value_t = i32::from(Zoned::now().year()), value_parser = clap::value_parser!(i32).range(1..=9999))]
        year: i32,
        #[arg(long, default_value = "EUR")]
        currency: String,
    },
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
        #[command(flatten)]
        date: AsOfArgs,
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
        /// Posting date in DD-MM; the year comes from the journal.
        #[arg(short, long, value_name = "DD-MM", default_value_t = Zoned::now().date().strftime("%d-%m").to_string())]
        date: String,
    },
}

#[derive(Args)]
#[group(multiple = false)]
pub(crate) struct AsOfArgs {
    /// Include all transactions through DD-MM in the journal year.
    #[arg(long, value_name = "DD-MM")]
    at: Option<String>,
    /// Include all transactions through the end of this month of the journal year.
    #[arg(short = 'm', long, value_parser = clap::value_parser!(u8).range(1..=12))]
    month: Option<u8>,
    /// Include all transactions through today; requires the current journal year.
    #[arg(long)]
    today: bool,
}

impl AsOfArgs {
    pub(crate) fn request(&self) -> DateRequest<'_> {
        DateRequest {
            to: self.at.as_deref(),
            shortcut: self
                .month
                .map(PeriodShortcut::Month)
                .or(self.today.then_some(PeriodShortcut::Today)),
            default_to_current_month: true,
            ..DateRequest::default()
        }
    }
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
            default_to_current_month: true,
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
