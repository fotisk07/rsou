use crate::{
    completion,
    dates::{DateRequest, PeriodShortcut},
};
use clap::{Args, Parser, Subcommand};
use clap_complete::ArgValueCompleter;
use jiff::Zoned;
use rust_decimal::Decimal;

#[derive(Parser)]
#[command(name = "sou")]
#[command(about = "Personal accounting CLI", version)]
pub(crate) struct Cli {
    /// Journal file.
    #[arg(short = 'j', long, global = true, default_value = "journal.sou")]
    pub(crate) journal_path: String,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Create a journal; never overwrite.
    #[command(display_order = 6)]
    Init {
        /// Journal name.
        #[arg(long, default_value = "Personal journal")]
        name: String,
        /// Journal year.
        #[arg(short = 'y', long, default_value_t = i32::from(Zoned::now().year()), value_parser = clap::value_parser!(i32).range(1..=9999))]
        year: i32,
        /// Three uppercase letters.
        #[arg(long, default_value = "EUR")]
        currency: String,
    },
    /// Sort transactions by date.
    #[command(display_order = 10)]
    Sort,
    /// Check journal validity.
    #[command(display_order = 5)]
    Check,
    /// Show profit and loss.
    #[command(
        display_order = 3,
        after_help = "Default: current month; other journal years: all dates."
    )]
    Pnl {
        #[command(flatten)]
        period: PeriodArgs,
        #[command(flatten)]
        shortcuts: PeriodShortcuts,
        /// Account levels; 0 = totals only.
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
    /// Show the balance sheet.
    #[command(
        display_order = 4,
        after_help = "Default: current month-end; other journal years: all dates."
    )]
    Bs {
        #[command(flatten)]
        date: AsOfArgs,
        /// Account levels; 0 = totals only.
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
    /// List the account tree.
    #[command(display_order = 1)]
    List,
    /// Add an account under an existing parent.
    #[command(display_order = 7)]
    Add {
        /// Full account path, e.g. Assets/Bank/Checking.
        #[arg(add = ArgValueCompleter::new(completion::account_parents))]
        account: String,
    },
    /// Show account transactions and balances.
    #[command(
        display_order = 2,
        after_help = "Default: current month; other journal years: all dates."
    )]
    Ledger {
        /// Account path, e.g. Assets/Bank.
        #[arg(add = ArgValueCompleter::new(completion::accounts))]
        account: String,
        #[command(flatten)]
        period: PeriodArgs,
        #[command(flatten)]
        shortcuts: PeriodShortcuts,
    },
    /// Record a transaction.
    #[command(display_order = 8)]
    Post {
        /// Amount to transfer; negatives reverse direction.
        #[arg(allow_negative_numbers = true)]
        amount: Decimal,
        /// Source account.
        #[arg(add = ArgValueCompleter::new(completion::accounts))]
        from_account: String,
        /// Destination account.
        #[arg(add = ArgValueCompleter::new(completion::accounts))]
        to_account: String,
        /// Description; quotes optional.
        #[arg(required = true, num_args = 1..)]
        description: Vec<String>,
        /// Posting date in the journal year.
        #[arg(short, long, value_name = "DD-MM", default_value_t = Zoned::now().date().strftime("%d-%m").to_string())]
        date: String,
    },
    /// Record one transaction with multiple postings.
    #[command(
        display_order = 9,
        after_help = "Example: sou split \"Grocery run\" Assets/Bank -50 Expenses/Food 40 Expenses/Household 10"
    )]
    Split {
        /// Description; quote if it contains spaces.
        description: String,
        /// Alternating account paths and signed amounts, which must sum to zero.
        #[arg(required = true, num_args = 1.., value_name = "ACCOUNT_AMOUNT", allow_negative_numbers = true)]
        postings: Vec<String>,
        /// Posting date in the journal year.
        #[arg(short, long, value_name = "DD-MM", default_value_t = Zoned::now().date().strftime("%d-%m").to_string())]
        date: String,
    },
}

#[derive(Args)]
#[group(multiple = false)]
pub(crate) struct AsOfArgs {
    /// Through this date, inclusive (journal year).
    #[arg(long, value_name = "DD-MM")]
    at: Option<String>,
    /// Through month-end (1-12, journal year).
    #[arg(short = 'm', long, value_parser = clap::value_parser!(u8).range(1..=12))]
    month: Option<u8>,
    /// Through today (current-year journals only).
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
    /// From this date, inclusive (journal year).
    #[arg(long, value_name = "DD-MM")]
    from: Option<String>,
    /// Through this date, inclusive (journal year).
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
    /// Whole month (1-12, journal year).
    #[arg(short = 'm', long, value_parser = clap::value_parser!(u8).range(1..=12))]
    month: Option<u8>,
    /// Whole quarter (1-4, journal year).
    #[arg(short = 'q', long, value_parser = clap::value_parser!(u8).range(1..=4))]
    quarter: Option<u8>,
    /// Month to date (current year only).
    #[arg(long)]
    mtd: bool,
    /// Quarter to date (current year only).
    #[arg(long)]
    qtd: bool,
    /// Year to date (current year only).
    #[arg(long)]
    ytd: bool,
    /// Previous whole month (current year only).
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
