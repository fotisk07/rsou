use crate::models::{Journal, Transaction};
use jiff::civil::Date;
use rust_decimal::Decimal;

pub struct LedgerEntry {
    pub date: Date,
    pub account: String,
    pub description: String,
    pub amount: Decimal,
    pub balance: Decimal,
}
/// Return chronological entries for this account and its descendants.
/// Each matching posting produces one entry. None means the account is unknown.
pub fn ledger_entries(journal: &Journal, path: &str) -> Option<Vec<LedgerEntry>> {
    journal.accounts.find_account(path)?;

    let child_prefix = format!("{path}::");
    let mut transactions: Vec<&Transaction> = journal.transactions.iter().collect();

    transactions.sort_by_key(|transaction| transaction.date);

    let mut entries = Vec::new();
    let mut balance = Decimal::ZERO;

    for transaction in transactions {
        for posting in &transaction.postings {
            if posting.account == path || posting.account.starts_with(&child_prefix) {
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

    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::ledger_entries;
    use crate::parser::parse_journal;
    use rust_decimal::Decimal;

    #[test]
    fn ledger_includes_descendants_and_running_balances() {
        let journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();

        let cash = ledger_entries(&journal, "Assets::Cash").unwrap();
        assert_eq!(cash.len(), 3);
        assert_eq!(cash[0].account, "Assets::Cash::Checking");
        assert_eq!(cash[0].balance, Decimal::from(200));
        assert_eq!(cash[1].amount, Decimal::from_str_exact("-23.75").unwrap());
        assert_eq!(cash[1].balance, Decimal::from_str_exact("176.25").unwrap());
        assert_eq!(cash[2].balance, Decimal::from_str_exact("181.25").unwrap());

        let groceries = ledger_entries(&journal, "Expenses::Food::Groceries").unwrap();
        assert_eq!(groceries.len(), 2);
        assert_eq!(groceries[0].description, "Groceries and snacks");
        assert_eq!(groceries[1].description, "Small refund");

        assert!(ledger_entries(&journal, "Expenses::Missing").is_none());
        assert!(ledger_entries(&journal, "Income").unwrap().is_empty());
    }

    #[test]
    fn ledger_sorts_by_date_without_changing_the_journal() {
        let mut journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();
        journal.transactions.reverse();

        let entries = ledger_entries(&journal, "Assets::Cash").unwrap();
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
            "\n2026-02-05 Foodie expense\n  Assets::Cash::Checking  -2.00\n  Expenses::Foodie  2.00\n",
        );
        let journal = parse_journal(&text).unwrap();

        let food = ledger_entries(&journal, "Expenses::Food").unwrap();
        assert_eq!(food.len(), 3); // Groceries, snacks, and the refund.
        assert_eq!(food[2].balance, Decimal::from_str_exact("18.75").unwrap());
        assert!(
            food.iter()
                .all(|transaction| transaction.description != "Foodie expense")
        );
    }
}
