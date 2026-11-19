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

pub struct PnlRow {
    pub account: String,
    pub depth: usize,
    pub amount: Decimal,
    pub direct: Decimal,
}

pub struct PnlReport {
    pub range: DateRange,
    pub rows: Vec<PnlRow>,
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
    let mut balances = HashMap::new();
    for transaction in range.transactions(journal) {
        for posting in &transaction.postings {
            *balances
                .entry(posting.account.as_str())
                .or_insert(Decimal::ZERO) += posting.amount;
        }
    }

    let mut rows = Vec::new();
    let mut net_result = Decimal::ZERO;
    for root in ["Income", "Expenses"] {
        let account = journal
            .accounts
            .find_account(root)
            .expect("built-in root account");
        let (total, mut section) = pnl_account(account, root, 0, &balances, keep_zero);
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

fn pnl_account(
    account: &Account,
    path: &str,
    depth: usize,
    balances: &HashMap<&str, Decimal>,
    keep_zero: bool,
) -> (Decimal, Vec<PnlRow>) {
    let mut total = balances.get(path).copied().unwrap_or(Decimal::ZERO);
    let mut descendants = Vec::new();
    for child in &account.children {
        let child_path = format!("{path}/{}", child.name);
        let (child_total, child_rows) =
            pnl_account(child, &child_path, depth + 1, balances, keep_zero);
        total += child_total;
        descendants.extend(child_rows);
    }

    let mut rows = Vec::new();
    // Breakdown columns need the complete tree so their rows stay aligned.
    if keep_zero || depth == 0 || !total.is_zero() {
        rows.push(PnlRow {
            account: account.name.clone(),
            depth,
            amount: total,
            direct: balances.get(path).copied().unwrap_or(Decimal::ZERO),
        });
    }
    rows.extend(descendants);
    (total, rows)
}

pub struct BsRow {
    pub account: String,
    pub depth: usize,
    pub amount: Decimal,
    pub direct: Decimal,
}

pub struct BsReport {
    pub range: DateRange,
    pub rows: Vec<BsRow>,
}

/// Summarize balances through the range's upper bound, including accumulated result in equity.
pub fn bs(journal: &Journal, range: DateRange) -> BsReport {
    // A balance sheet never has a lower bound, even if a caller supplies one.
    let range = DateRange {
        from: None,
        to: range.to,
    };
    let mut balances = HashMap::new();
    for transaction in range.transactions(journal) {
        for posting in &transaction.postings {
            *balances
                .entry(posting.account.as_str())
                .or_insert(Decimal::ZERO) += posting.amount;
        }
    }

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
        let (_, mut section) = bs_account(account, root, 0, &balances);
        if root != "Assets" {
            for row in &mut section {
                row.amount = -row.amount;
                row.direct = -row.direct;
            }
        }
        rows.extend(section);
        if root == "Equity" && !net_result.is_zero() {
            rows.push(BsRow {
                account: "Net Result".to_string(),
                depth: 1,
                amount: net_result,
                direct: net_result,
            });
        }
    }

    BsReport { range, rows }
}

fn bs_account(
    account: &Account,
    path: &str,
    depth: usize,
    balances: &HashMap<&str, Decimal>,
) -> (Decimal, Vec<BsRow>) {
    let mut total = balances.get(path).copied().unwrap_or(Decimal::ZERO);
    let mut descendants = Vec::new();
    for child in &account.children {
        let child_path = format!("{path}/{}", child.name);
        let (child_total, child_rows) = bs_account(child, &child_path, depth + 1, balances);
        total += child_total;
        descendants.extend(child_rows);
    }

    let mut rows = Vec::new();
    // Keep section boundaries even when a root totals zero; hide zero detail rows.
    if depth == 0 || !total.is_zero() {
        rows.push(BsRow {
            account: account.name.clone(),
            depth,
            amount: total,
            direct: balances.get(path).copied().unwrap_or(Decimal::ZERO),
        });
    }
    rows.extend(descendants);
    (total, rows)
}
