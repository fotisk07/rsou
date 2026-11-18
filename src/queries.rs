use crate::dates::DateRange;
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
        let (total, mut section) = pnl_account(account, root, 0, &balances);
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
) -> (Decimal, Vec<PnlRow>) {
    let mut total = balances.get(path).copied().unwrap_or(Decimal::ZERO);
    let mut descendants = Vec::new();
    for child in &account.children {
        let child_path = format!("{path}/{}", child.name);
        let (child_total, child_rows) = pnl_account(child, &child_path, depth + 1, balances);
        total += child_total;
        descendants.extend(child_rows);
    }

    let mut rows = Vec::new();
    // Keep section boundaries even when a root totals zero; hide zero detail rows.
    if depth == 0 || !total.is_zero() {
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

#[cfg(test)]
mod tests {
    use super::ledger;
    use crate::{dates::DateRange, parser::parse_journal};
    use rust_decimal::Decimal;

    #[test]
    fn ledger_includes_descendants_and_running_balances() {
        let journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();

        let cash = ledger(&journal, "Assets/Cash", DateRange::default())
            .unwrap()
            .entries;
        assert_eq!(cash.len(), 3);
        assert_eq!(cash[0].account, "Assets/Cash/Checking");
        assert_eq!(cash[0].balance, Decimal::from(200));
        assert_eq!(cash[1].amount, Decimal::from_str_exact("-23.75").unwrap());
        assert_eq!(cash[1].balance, Decimal::from_str_exact("176.25").unwrap());
        assert_eq!(cash[2].balance, Decimal::from_str_exact("181.25").unwrap());

        let groceries = ledger(&journal, "Expenses/Food/Groceries", DateRange::default())
            .unwrap()
            .entries;
        assert_eq!(groceries.len(), 2);
        assert_eq!(groceries[0].description, "Groceries and snacks");
        assert_eq!(groceries[1].description, "Small refund");

        assert!(ledger(&journal, "Expenses/Missing", DateRange::default()).is_none());
        assert!(
            ledger(&journal, "Income", DateRange::default())
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn ledger_sorts_by_date_without_changing_the_journal() {
        let mut journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();
        journal.transactions.reverse();

        let entries = ledger(&journal, "Assets/Cash", DateRange::default())
            .unwrap()
            .entries;
        assert_eq!(entries[0].description, "Opening balance");
        assert_eq!(entries[1].description, "Groceries and snacks");
        assert_eq!(entries[2].description, "Small refund");
        assert_eq!(
            entries[2].balance,
            Decimal::from_str_exact("181.25").unwrap()
        );
        assert_eq!(journal.transactions[0].description, "Small refund");
    }

    #[test]
    fn ledger_does_not_match_similar_account_names() {
        let mut text =
            include_str!("../test_data/split.sou").replace("    Snacks", "    Snacks\n  Foodie");
        text.push_str(
            "\n2026-02-05 Foodie expense\n  Assets/Cash/Checking  -2.00\n  Expenses/Foodie  2.00\n",
        );
        let journal = parse_journal(&text).unwrap();

        let food = ledger(&journal, "Expenses/Food", DateRange::default())
            .unwrap()
            .entries;
        assert_eq!(food.len(), 3); // Groceries, snacks, and the refund.
        assert_eq!(food[2].balance, Decimal::from_str_exact("18.75").unwrap());
        assert!(
            food.iter()
                .all(|transaction| transaction.description != "Foodie expense")
        );
    }
}
