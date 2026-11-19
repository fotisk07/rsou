use crate::{
    completion,
    dates::{DateRequest, PeriodShortcut},
    models::RecurrenceFrequency,
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
    #[arg(short = 'j', long, global = true, default_value = "journal.sou", display_order = 100)]
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
        /// Break the selected period into monthly columns.
        #[arg(long, conflicts_with = "by_quarter", display_order = 50)]
        by_month: bool,
        /// Break the selected period into quarterly columns.
        #[arg(long, display_order = 51)]
        by_quarter: bool,
        /// Account levels; 0 = totals only.
        #[arg(long, default_value_t = 1, display_order = 52)]
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
        #[arg(long, default_value_t = 1, display_order = 52)]
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
    /// Manage recurring transaction templates.
    Recur {
        #[command(subcommand)]
        command: RecurCommands,
    },
    /// Record a transaction.
    #[command(display_order = 8)]
    Post {
        /// Amount to transfer; negatives reverse direction.
        #[arg(allow_negative_numbers = true)]
        amount: Decimal,
        /// Source account path or alias.
        #[arg(add = ArgValueCompleter::new(completion::posting_accounts))]
        from_account: String,
        /// Destination account path or alias.
        #[arg(add = ArgValueCompleter::new(completion::posting_accounts))]
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

#[derive(Subcommand)]
pub(crate) enum RecurCommands {
    /// List recurring templates and their next dates.
    List {
        /// Include start dates and posting splits.
        #[arg(long)]
        details: bool,
    },
    /// Catch up recurring transactions through today by default.
    Post {
        #[command(flatten)]
        cutoff: RecurringPostDates,
        /// Show transactions without saving or advancing next dates.
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a balanced recurring template; does not post transactions.
    #[command(
        after_help = "Example: sou recur add Rent Assets/Bank -800 Expenses/Rent 800 --start 01-01\nDefault frequency: monthly."
    )]
    Add {
        /// Description; quote if it contains spaces.
        description: String,
        /// Alternating account paths and signed amounts, which must sum to zero.
        #[arg(required = true, num_args = 1.., value_name = "ACCOUNT_AMOUNT", allow_negative_numbers = true)]
        postings: Vec<String>,
        /// First occurrence in the journal year (defaults to today's day and month).
        #[arg(long, value_name = "DD-MM", default_value_t = Zoned::now().date().strftime("%d-%m").to_string())]
        start: String,
        #[command(flatten)]
        frequency: RecurrenceArgs,
    },
}

#[derive(Args)]
#[group(multiple = false)]
pub(crate) struct RecurringPostDates {
    /// Catch up through Sunday, capped at journal year-end (current year only).
    #[arg(long)]
    week: bool,
    /// Catch up through this month-end (current year only).
    #[arg(long)]
    month: bool,
    /// Catch up through this date, inclusive (journal year).
    #[arg(short, long, value_name = "DD-MM")]
    date: Option<String>,
}

impl RecurringPostDates {
    pub(crate) fn request(&self) -> DateRequest<'_> {
        DateRequest {
            to: self.date.as_deref(),
            shortcut: if self.date.is_some() {
                None
            } else if self.week {
                Some(PeriodShortcut::WeekEnd)
            } else if self.month {
                Some(PeriodShortcut::MonthEnd)
            } else {
                Some(PeriodShortcut::Today)
            },
            ..DateRequest::default()
        }
    }
}

#[derive(Args)]
#[group(multiple = false)]
pub(crate) struct RecurrenceArgs {
    /// Repeat every day.
    #[arg(long)]
    day: bool,
    /// Repeat every week, anchored to the start date.
    #[arg(long)]
    week: bool,
    /// Repeat every two weeks, not twice a month.
    #[arg(long)]
    biweekly: bool,
    /// Repeat every month (default), anchored to the start date.
    #[arg(long)]
    month: bool,
}

impl RecurrenceArgs {
    pub(crate) fn frequency(&self) -> RecurrenceFrequency {
        if self.day {
            RecurrenceFrequency::Daily
        } else if self.week {
            RecurrenceFrequency::Weekly
        } else if self.biweekly {
            RecurrenceFrequency::Biweekly
        } else {
            RecurrenceFrequency::Monthly
        }
    }
}

#[derive(Args)]
#[group(multiple = false)]
pub(crate) struct AsOfArgs {
    /// Through December 31 of the journal year.
    #[arg(long, display_order = 10)]
    year: bool,
    /// Through this date, inclusive (journal year).
    #[arg(long, value_name = "DD-MM", display_order = 30)]
    at: Option<String>,
    /// Through month-end; omit the number for the current month (current year only).
    #[arg(short = 'm', long, num_args = 0..=1, value_parser = clap::value_parser!(u8).range(1..=12), display_order = 11)]
    month: Option<Option<u8>>,
    /// Through today (current-year journals only).
    #[arg(long, display_order = 20)]
    today: bool,
}

impl AsOfArgs {
    pub(crate) fn request(&self) -> DateRequest<'_> {
        DateRequest {
            to: self.at.as_deref(),
            shortcut: self
                .month
                .map(|month| month.map_or(PeriodShortcut::CurrentMonth, PeriodShortcut::Month))
                .or(self.today.then_some(PeriodShortcut::Today))
                .or(self.year.then_some(PeriodShortcut::Year)),
            default_to_current_month: true,
            ..DateRequest::default()
        }
    }
}

#[derive(Args)]
pub(crate) struct PeriodArgs {
    /// From this date, inclusive (journal year).
    #[arg(long, value_name = "DD-MM", display_order = 30)]
    from: Option<String>,
    /// Through this date, inclusive (journal year).
    #[arg(long, value_name = "DD-MM", display_order = 31)]
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
    /// Whole journal year.
    #[arg(long, display_order = 10)]
    year: bool,
    /// Whole month (1-12); omit the number for the current month (current year only).
    #[arg(short = 'm', long, num_args = 0..=1, value_parser = clap::value_parser!(u8).range(1..=12), display_order = 11)]
    month: Option<Option<u8>>,
    /// Whole quarter (1-4); omit the number for the current quarter (current year only).
    #[arg(short = 'q', long, num_args = 0..=1, value_parser = clap::value_parser!(u8).range(1..=4), display_order = 12)]
    quarter: Option<Option<u8>>,
    /// Month to date (current year only).
    #[arg(long, display_order = 20)]
    mtd: bool,
    /// Quarter to date (current year only).
    #[arg(long, display_order = 21)]
    qtd: bool,
    /// Year to date (current year only).
    #[arg(long, display_order = 22)]
    ytd: bool,
    /// Previous whole month (current year only).
    #[arg(long, display_order = 13)]
    last_month: bool,
}

impl PeriodShortcuts {
    pub(crate) fn selection(&self) -> Option<PeriodShortcut> {
        match self {
            Self {
                month: Some(month), ..
            } => Some(month.map_or(PeriodShortcut::CurrentMonth, PeriodShortcut::Month)),
            Self {
                quarter: Some(quarter),
                ..
            } => Some(quarter.map_or(PeriodShortcut::CurrentQuarter, PeriodShortcut::Quarter)),
            Self { year: true, .. } => Some(PeriodShortcut::Year),
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

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::{CommandFactory, Parser};
    use jiff::civil::Date;

    #[test]
    fn report_help_keeps_date_display_and_global_options_in_order() {
        for (command, options) in [
            ("pnl", vec!["--year", "--month", "--quarter", "--last-month", "--mtd", "--qtd", "--ytd", "--from", "--to", "--by-month", "--by-quarter", "--depth", "--journal-path", "--help"]),
            ("bs", vec!["--year", "--month", "--today", "--at", "--depth", "--journal-path", "--help"]),
            ("ledger", vec!["--year", "--month", "--quarter", "--last-month", "--mtd", "--qtd", "--ytd", "--from", "--to", "--journal-path", "--help"]),
        ] {
            let mut cli = Cli::command();
            cli.build();
            let help = cli.find_subcommand_mut(command).unwrap().render_help().to_string();
            let positions: Vec<_> = options.iter().map(|option| help.find(option).unwrap()).collect();
            assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{help}");
        }
    }

    #[test]
    fn optional_period_numbers_select_current_or_explicit_periods() {
        let today = Date::new(2026, 6, 15).unwrap();
        for (args, from, to, relative) in [
            (vec!["pnl", "--month"], (6, 1), (6, 30), true),
            (vec!["pnl", "-m"], (6, 1), (6, 30), true),
            (vec!["pnl", "--quarter"], (4, 1), (6, 30), true),
            (vec!["pnl", "-q"], (4, 1), (6, 30), true),
            (vec!["pnl", "--month", "--by-month"], (6, 1), (6, 30), true),
            (vec!["pnl", "--quarter", "--by-quarter"], (4, 1), (6, 30), true),
            (vec!["bs", "--month"], (6, 1), (6, 30), true),
            (vec!["bs", "--month", "2"], (2, 1), (2, 28), false),
            (vec!["pnl", "--month", "3"], (3, 1), (3, 31), false),
            (vec!["pnl", "--quarter", "4"], (10, 1), (12, 31), false),
        ] {
            let cli = Cli::try_parse_from(std::iter::once("sou").chain(args)).unwrap();
            for year in [2025, 2026] {
                let request = match &cli.command {
                    Commands::Pnl { period, shortcuts, .. } => period.request(shortcuts.selection()),
                    Commands::Bs { date, .. } => date.request(),
                    _ => unreachable!(),
                };
                let result = request.resolve(year, today);
                if relative && year != 2026 {
                    assert!(result.err().unwrap().contains("relative date shortcuts"));
                } else {
                    let range = result.unwrap();
                    assert_eq!(range.from, Some(Date::new(year as i16, from.0, from.1).unwrap()));
                    assert_eq!(range.to, Some(Date::new(year as i16, to.0, to.1).unwrap()));
                }
            }
        }
        for args in [
            vec!["pnl", "--month", "0"],
            vec!["pnl", "--month", "13"],
            vec!["pnl", "--quarter", "5"],
            vec!["pnl", "--month", "--quarter"],
            vec!["pnl", "--month", "--from", "01-01"],
            vec!["bs", "--month", "--today"],
        ] {
            assert!(Cli::try_parse_from(std::iter::once("sou").chain(args)).is_err());
        }
    }

    #[test]
    fn year_selects_whole_journal_year_for_pnl_and_bs() {
        let today = Date::new(2026, 6, 15).unwrap();
        for year in [2024, 2026, 2027] {
            for command in ["pnl", "bs"] {
                let cli = Cli::try_parse_from(["sou", command, "--year"]).unwrap();
                let request = match &cli.command {
                    Commands::Pnl { period, shortcuts, .. } => period.request(shortcuts.selection()),
                    Commands::Bs { date, .. } => date.request(),
                    _ => unreachable!(),
                };
                let range = request.resolve(year, today).unwrap();
                assert_eq!(range.from, Some(Date::new(year as i16, 1, 1).unwrap()));
                assert_eq!(range.to, Some(Date::new(year as i16, 12, 31).unwrap()));
            }
        }
    }

    #[test]
    fn year_conflicts_with_other_date_selections_but_allows_breakdowns() {
        for args in [
            vec!["pnl", "--year", "--month", "1"],
            vec!["pnl", "--year", "--quarter", "1"],
            vec!["pnl", "--year", "--ytd"],
            vec!["pnl", "--year", "--from", "01-01"],
            vec!["pnl", "--year", "--to", "31-12"],
            vec!["bs", "--year", "--month", "1"],
            vec!["bs", "--year", "--at", "31-12"],
            vec!["bs", "--year", "--today"],
        ] {
            assert!(Cli::try_parse_from(std::iter::once("sou").chain(args)).is_err());
        }
        for breakdown in ["--by-month", "--by-quarter"] {
            assert!(Cli::try_parse_from(["sou", "pnl", "--year", breakdown]).is_ok());
        }
    }
}
