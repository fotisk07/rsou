use crate::models::{Journal, Transaction};

/// Find transactions posted to this account or any of its descendants.
/// Returns None when the account does not exist.
pub fn transactions_for<'a>(journal: &'a Journal, path: &str) -> Option<Vec<&'a Transaction>> {
    journal.accounts.find_account(path)?;

    let child_prefix = format!("{path}::");
    Some(
        journal
            .transactions
            .iter()
            .filter(|transaction| {
                transaction.postings.iter().any(|posting| {
                    posting.account == path || posting.account.starts_with(&child_prefix)
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::transactions_for;
    use crate::parser::parse_journal;

    #[test]
    fn test_ledger() {
        let journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();

        let cash = transactions_for(&journal, "Assets::Cash").unwrap();
        assert_eq!(cash.len(), 3);

        let groceries = transactions_for(&journal, "Expenses::Food::Groceries").unwrap();
        assert_eq!(groceries.len(), 2);
        assert_eq!(groceries[0].description, "Groceries and snacks");
        assert_eq!(groceries[1].description, "Small refund");

        assert!(transactions_for(&journal, "Expenses::Missing").is_none());
        assert!(transactions_for(&journal, "Income").unwrap().is_empty());
    }

    #[test]
    fn ledger_does_not_match_similar_account_names() {
        let mut text =
            include_str!("../test_data/split.sou").replace("    Snacks", "    Snacks\n  Foodie");
        text.push_str(
            "\n2026-02-05 Foodie expense\n  Assets::Cash::Checking  -2.00\n  Expenses::Foodie  2.00\n",
        );
        let journal = parse_journal(&text).unwrap();

        let food = transactions_for(&journal, "Expenses::Food").unwrap();
        assert_eq!(food.len(), 2);
        assert!(
            food.iter()
                .all(|transaction| transaction.description != "Foodie expense")
        );
    }
}
