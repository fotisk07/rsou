use crate::dates::{DateRange, PeriodBreakdown};
use crate::models::{Account, Journal, Transaction};
use jiff::civil::Date;
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct LedgerEntry {
    pub date: Date,
    pub account: String,
    pub description: String,
    pub amount: Decimal,
    pub balance: Decimal,
}
pub struct LedgerReport {
    pub range: DateRange,
    pub opening: Decimal,
    pub entries: Vec<LedgerEntry>,
}

/// Return chronological entries and the pre-period balance for an account and its descendants.
/// Each matching posting produces one entry. None means the account is unknown.
pub fn ledger(journal: &Journal, path: &str, range: DateRange) -> Option<LedgerReport> {
    journal.accounts.find_account(path)?;

    let child_prefix = format!("{path}/");
    let matches_account = |account: &str| account == path || account.starts_with(&child_prefix);
    let opening = journal
        .transactions
        .iter()
        .filter(|transaction| range.before_start(transaction.date))
        .flat_map(|transaction| &transaction.postings)
        .filter(|posting| matches_account(&posting.account))
        .map(|posting| posting.amount)
        .sum();
    let mut transactions: Vec<&Transaction> = range.transactions(journal).collect();

    transactions.sort_by_key(|transaction| transaction.date);

    let mut entries = Vec::new();
    let mut balance = opening;

    for transaction in transactions {
        for posting in &transaction.postings {
            if matches_account(&posting.account) {
                balance += posting.amount;

                entries.push(LedgerEntry {
                    date: transaction.date,
                    account: posting.account.clone(),
                    description: transaction.description.clone(),
                    amount: posting.amount,
                    balance,
                });
            }
        }
    }

    Some(LedgerReport {
        range,
        opening,
        entries,
    })
}

pub struct AccountBalanceRow {
    pub account: String,
    pub depth: usize,
    pub amount: Decimal,
    pub direct: Decimal,
}

pub struct PnlReport {
    pub range: DateRange,
    pub rows: Vec<AccountBalanceRow>,
    pub net_result: Decimal,
}

/// Summarize income and expenses within an inclusive date range without journal mutation.
pub fn pnl(journal: &Journal, range: DateRange) -> PnlReport {
    pnl_report(journal, range, false)
}

/// Keep an identical account tree in every column, including accounts whose
/// amounts cancel across periods. The formatter hides rows empty in all columns.
pub fn pnl_breakdown(
    journal: &Journal,
    range: DateRange,
    breakdown: PeriodBreakdown,
) -> Result<Vec<(String, PnlReport)>, String> {
    let mut columns: Vec<_> = breakdown
        .periods(range, journal.metadata.year)?
        .into_iter()
        .map(|(label, period)| (label, pnl_report(journal, period, true)))
        .collect();
    columns.push(("Total".to_string(), pnl_report(journal, range, true)));
    Ok(columns)
}

fn pnl_report(journal: &Journal, range: DateRange, keep_zero: bool) -> PnlReport {
    let balances = collect_balances(journal, range);

    let mut rows = Vec::new();
    let mut net_result = Decimal::ZERO;
    for root in ["Income", "Expenses"] {
        let account = journal
            .accounts
            .find_account(root)
            .expect("built-in root account");
        let (total, mut section) = account_balances(account, root, 0, &balances, keep_zero);
        // Both income credits and expense debits reduce the raw accounting result.
        net_result -= total;
        if root == "Income" {
            for row in &mut section {
                row.amount = -row.amount;
                row.direct = -row.direct;
            }
        }
        rows.extend(section);
    }

    PnlReport {
        range,
        rows,
        net_result,
    }
}

fn account_balances(
    account: &Account,
    path: &str,
    depth: usize,
    balances: &HashMap<&str, Decimal>,
    keep_zero: bool,
) -> (Decimal, Vec<AccountBalanceRow>) {
    let mut total = balances.get(path).copied().unwrap_or(Decimal::ZERO);
    let mut descendants = Vec::new();
    for child in &account.children {
        let child_path = format!("{path}/{}", child.name);
        let (child_total, child_rows) =
            account_balances(child, &child_path, depth + 1, balances, keep_zero);
        total += child_total;
        descendants.extend(child_rows);
    }

    let mut rows = Vec::new();
    // Keep ancestors of active children even when their amounts cancel out.
    // Breakdown columns need the complete tree so their rows stay aligned.
    if keep_zero || depth == 0 || !total.is_zero() || !descendants.is_empty() {
        rows.push(AccountBalanceRow {
            account: account.name.clone(),
            depth,
            amount: total,
            direct: balances.get(path).copied().unwrap_or(Decimal::ZERO),
        });
    }
    rows.extend(descendants);
    (total, rows)
}

pub struct BsReport {
    pub range: DateRange,
    pub rows: Vec<AccountBalanceRow>,
}

/// Summarize balances through the range's upper bound, including accumulated result in equity.
pub fn bs(journal: &Journal, range: DateRange) -> BsReport {
    bs_report(journal, range, false)
}

/// Cumulative snapshots at each period end, with identical account trees.
/// Unlike P&L breakdowns, snapshots must never be summed into a Total column.
pub fn bs_breakdown(
    journal: &Journal,
    range: DateRange,
    breakdown: PeriodBreakdown,
) -> Result<Vec<(String, BsReport)>, String> {
    breakdown
        .periods(range, journal.metadata.year)?
        .into_iter()
        .map(|(_, period)| {
            let cutoff = period.to.expect("periods have an end date");
            Ok((
                cutoff.strftime("%d %b %Y").to_string(),
                bs_report(journal, period, true),
            ))
        })
        .collect()
}

fn bs_report(journal: &Journal, range: DateRange, keep_zero: bool) -> BsReport {
    // A balance sheet never has a lower bound, even if a caller supplies one.
    let range = DateRange {
        from: None,
        to: range.to,
    };
    let mut balances = collect_balances(journal, range);

    // Use the same unbounded-start selection: equity needs accumulated, not period profit.
    let net_result = pnl(journal, range).net_result;
    // Include the calculated result in equity's total without changing the journal.
    *balances.entry("Equity").or_insert(Decimal::ZERO) -= net_result;

    let mut rows = Vec::new();
    for root in ["Assets", "Liabilities", "Equity"] {
        let account = journal
            .accounts
            .find_account(root)
            .expect("built-in root account");
        let (_, mut section) = account_balances(account, root, 0, &balances, keep_zero);
        if root != "Assets" {
            for row in &mut section {
                row.amount = -row.amount;
                row.direct = -row.direct;
            }
        }
        rows.extend(section);
        if root == "Equity" && (keep_zero || !net_result.is_zero()) {
            rows.push(AccountBalanceRow {
                account: "Net Result".to_string(),
                depth: 1,
                amount: net_result,
                direct: net_result,
            });
        }
    }

    BsReport { range, rows }
}

fn collect_balances(journal: &Journal, range: DateRange) -> HashMap<&str, Decimal> {
    let mut balances = HashMap::new();
    for transaction in range.transactions(journal) {
        for posting in &transaction.postings {
            *balances
                .entry(posting.account.as_str())
                .or_insert(Decimal::ZERO) += posting.amount;
        }
    }
    balances
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{journal::parse_journal, validation::validate_journal};

    fn journal(transactions: &str) -> Journal {
        let journal = parse_journal(&format!(
            "[JOURNAL]\nname: Tests\nauthor:\nyear: 2024\ncurrency: EUR\n\
             [ACCOUNTS]\nAssets\n  Bank\n    Checking\n  BankOther\n\
             Liabilities\n  Loan\nEquity\n  Capital\nIncome\n  Sales\n\
             Expenses\n  Food\n[TRANSACTIONS]\n{transactions}"
        ))
        .unwrap();
        validate_journal(&journal).unwrap();
        journal
    }

    #[test]
    fn balance_sheet_snapshots_are_cumulative_aligned_and_stop_at_exact_cutoff() {
        let journal = journal(
            "2024-01-01 Capital\n  Assets/Bank 1000\n  Equity/Capital -1000\n\
             2024-02-29 Income\n  Assets/Bank 100\n  Income/Sales -100\n\
             2024-03-31 Expense\n  Assets/Bank -100\n  Expenses/Food 100\n\
             2024-05-15 Borrowing\n  Assets/Bank 500\n  Liabilities/Loan -500\n\
             2024-05-16 Future income\n  Assets/Bank 900\n  Income/Sales -900\n",
        );
        let range = DateRange {
            from: Some("2024-02-10".parse().unwrap()),
            to: Some("2024-05-15".parse().unwrap()),
        };
        let monthly = bs_breakdown(&journal, range, PeriodBreakdown::Month).unwrap();
        assert_eq!(
            monthly
                .iter()
                .map(|(label, _)| label.as_str())
                .collect::<Vec<_>>(),
            ["29 Feb 2024", "31 Mar 2024", "30 Apr 2024", "15 May 2024"]
        );
        let quarterly = bs_breakdown(&journal, range, PeriodBreakdown::Quarter).unwrap();
        assert_eq!(
            quarterly
                .iter()
                .map(|(label, _)| label.as_str())
                .collect::<Vec<_>>(),
            ["31 Mar 2024", "15 May 2024"]
        );
        let amount = |report: &BsReport, name: &str| {
            report
                .rows
                .iter()
                .find(|row| row.account == name)
                .unwrap()
                .amount
        };
        assert_eq!(amount(&monthly[0].1, "Assets"), Decimal::from(1100));
        assert_eq!(amount(&monthly[0].1, "Net Result"), Decimal::from(100));
        assert_eq!(amount(&monthly[1].1, "Net Result"), Decimal::ZERO);
        for columns in [&monthly, &quarterly] {
            let keys = |report: &BsReport| {
                report
                    .rows
                    .iter()
                    .map(|row| (row.account.clone(), row.depth))
                    .collect::<Vec<_>>()
            };
            for (_, report) in columns {
                assert_eq!(keys(report), keys(&columns[0].1));
                assert!(report.range.from.is_none());
                assert_eq!(
                    amount(report, "Assets"),
                    amount(report, "Liabilities") + amount(report, "Equity")
                );
            }
            assert_eq!(
                amount(&columns.last().unwrap().1, "Assets"),
                Decimal::from(1500)
            );
        }
    }

    #[test]
    fn breakdowns_keep_rows_aligned_when_period_amounts_cancel() {
        let journal = journal(
            "2024-01-10 Purchase\n  Assets/Bank -10\n  Expenses/Food 10\n\
             2024-02-10 Refund\n  Assets/Bank 10\n  Expenses/Food -10\n",
        );
        let range = DateRange {
            from: Some("2024-01-01".parse().unwrap()),
            to: Some("2024-06-30".parse().unwrap()),
        };
        for breakdown in [PeriodBreakdown::Month, PeriodBreakdown::Quarter] {
            let columns = pnl_breakdown(&journal, range, breakdown).unwrap();
            let keys = |report: &PnlReport| {
                report
                    .rows
                    .iter()
                    .map(|row| (row.account.clone(), row.depth))
                    .collect::<Vec<_>>()
            };
            for (_, report) in &columns {
                assert_eq!(keys(report), keys(&columns[0].1));
            }
            let total = &columns.last().unwrap().1;
            let food = total.rows.iter().find(|row| row.account == "Food").unwrap();
            assert_eq!(food.amount, Decimal::ZERO);
            for (index, row) in total.rows.iter().enumerate() {
                assert_eq!(
                    row.amount,
                    columns[..columns.len() - 1]
                        .iter()
                        .map(|(_, report)| report.rows[index].amount)
                        .sum::<Decimal>()
                );
            }
        }
    }

    #[test]
    fn pnl_keeps_zero_total_parents_with_active_children_but_hides_unused_accounts() {
        let journal = journal("2024-01-10 Reclassify\n  Expenses -10\n  Expenses/Food 10\n");
        let report = pnl(&journal, DateRange::default());
        assert_eq!(
            report
                .rows
                .iter()
                .map(|row| row.account.as_str())
                .collect::<Vec<_>>(),
            vec!["Income", "Expenses", "Food"]
        );
        assert_eq!(report.rows[1].amount, Decimal::ZERO);
        assert_eq!(report.rows[1].direct, Decimal::from(-10));
        assert_eq!(report.rows[2].amount, Decimal::from(10));
    }

    #[test]
    fn opening_and_running_balances_include_descendants_but_not_similar_names() {
        let journal = journal(
            "2024-03-01 After cutoff
  Assets/Bank 900
  Equity -900
2024-02-29 Closing boundary
  Assets/Bank/Checking -25.25
  Equity 25.25
2024-01-31 Opening
  Assets/Bank/Checking 100.10
  Equity -100.10
2024-02-01 Starting boundary
  Assets/Bank 10.05
  Assets/Bank/Checking 2.10
  Equity -12.15
2024-01-01 Unrelated opening
  Assets/BankOther 500
  Equity -500
2024-02-15 Unrelated movement
  Assets/BankOther 700
  Equity -700
",
        );
        let report = ledger(
            &journal,
            "Assets/Bank",
            DateRange {
                from: Some("2024-02-01".parse().unwrap()),
                to: Some("2024-02-29".parse().unwrap()),
            },
        )
        .unwrap();

        assert_eq!(report.opening, Decimal::new(10010, 2));
        let amounts: Vec<_> = report
            .entries
            .iter()
            .map(|e| (e.amount, e.balance))
            .collect();
        assert_eq!(
            amounts,
            vec![
                (Decimal::new(1005, 2), Decimal::new(11015, 2)),
                (Decimal::new(210, 2), Decimal::new(11225, 2)),
                (Decimal::new(-2525, 2), Decimal::new(8700, 2)),
            ]
        );
    }

    #[test]
    fn period_profit_rolls_up_direct_and_child_postings_with_income_credit_signs() {
        let journal = journal(
            "2024-01-31 Prior income
  Income -1000
  Assets 1000
2024-02-01 Income
  Income -20.10
  Income/Sales -100.25
  Assets 120.35
2024-02-29 Expense and refund
  Expenses 10.05
  Expenses/Food 30.20
  Expenses/Food -5.10
  Assets -35.15
2024-03-01 Later expense
  Expenses 500
  Assets -500
",
        );
        let report = pnl(
            &journal,
            DateRange {
                from: Some("2024-02-01".parse().unwrap()),
                to: Some("2024-02-29".parse().unwrap()),
            },
        );
        let row = |name| report.rows.iter().find(|r| r.account == name).unwrap();
        assert_eq!(row("Income").amount, Decimal::new(12035, 2));
        assert_eq!(row("Income").direct, Decimal::new(2010, 2));
        assert_eq!(row("Sales").amount, Decimal::new(10025, 2));
        assert_eq!(row("Expenses").amount, Decimal::new(3515, 2));
        assert_eq!(row("Expenses").direct, Decimal::new(1005, 2));
        assert_eq!(row("Food").amount, Decimal::new(2510, 2));
        assert_eq!(report.net_result, Decimal::new(8520, 2));
    }

    #[test]
    fn zero_total_parent_is_kept_when_direct_and_child_balances_cancel() {
        let journal =
            journal("2024-02-01 Internal transfer\n  Assets/Bank -10\n  Assets/Bank/Checking 10\n");
        let report = bs(
            &journal,
            DateRange {
                from: None,
                to: None,
            },
        );
        let bank = report
            .rows
            .iter()
            .find(|row| row.account == "Bank")
            .unwrap();
        assert_eq!(bank.amount, Decimal::ZERO);
        assert_eq!(bank.direct, Decimal::from(-10));
        assert_eq!(
            report
                .rows
                .iter()
                .find(|row| row.account == "Checking")
                .unwrap()
                .amount,
            Decimal::from(10)
        );
    }

    #[test]
    fn cutoff_includes_accumulated_profit_in_equity_and_preserves_accounting_equation() {
        let journal = journal(
            "2024-01-01 Capital and borrowing
  Assets/Bank 1500
  Equity/Capital -1000
  Liabilities/Loan -500
2024-01-15 Prior profit
  Assets/Bank 200
  Income -200
2024-02-29 Current expense
  Assets/Bank -50
  Expenses 50
2024-03-01 Future income
  Assets/Bank 900
  Income -900
",
        );
        let report = bs(
            &journal,
            DateRange {
                from: Some("2024-02-01".parse().unwrap()),
                to: Some("2024-02-29".parse().unwrap()),
            },
        );
        let amount = |name| {
            report
                .rows
                .iter()
                .find(|r| r.account == name)
                .unwrap()
                .amount
        };
        assert_eq!(amount("Assets"), Decimal::from(1650));
        assert_eq!(amount("Liabilities"), Decimal::from(500));
        assert_eq!(amount("Capital"), Decimal::from(1000));
        assert_eq!(amount("Net Result"), Decimal::from(150));
        assert_eq!(amount("Equity"), Decimal::from(1150));
        assert_eq!(amount("Assets"), amount("Liabilities") + amount("Equity"));
        // Repeating the report must not accumulate synthetic profit in the journal.
        let again = bs(&journal, report.range);
        assert_eq!(
            again
                .rows
                .iter()
                .find(|r| r.account == "Equity")
                .unwrap()
                .amount,
            amount("Equity")
        );
    }
}
