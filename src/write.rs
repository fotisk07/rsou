use crate::models::{Account, Journal, Transaction};

fn format_account(account: &Account, depth: usize, text: &mut String) {
    text.push_str(&"  ".repeat(depth));
    text.push_str(&account.name);
    text.push('\n');

    for child in &account.children {
        format_account(child, depth + 1, text);
    }
}
fn format_transaction(transaction: &Transaction, text: &mut String) {
    text.push_str(&format!(
        "{} {}\n",
        transaction.date, transaction.description,
    ));

    for posting in &transaction.postings {
        text.push_str(&format!("  {}  {:.2}\n", posting.account, posting.amount,));
    }
}

pub fn format_journal(journal: &Journal) -> String {
    // Metadata
    let mut text = format!(
        "[JOURNAL]\n\nname: {}\nyear: {}\ncurrency: {}\n\n",
        journal.metadata.name, journal.metadata.year, journal.metadata.currency,
    );

    // Accounts
    text.push_str("[ACCOUNTS]\n\n");
    for root_name in ["Assets", "Liabilities", "Equity", "Income", "Expenses"] {
        let root = journal
            .accounts
            .find_account(root_name)
            .expect("root accounts always exist");

        format_account(root, 0, &mut text);
    }

    // Transactions
    text.push('\n');
    text.push_str("[TRANSACTIONS]\n\n");
    for (index, transaction) in journal.transactions.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        format_transaction(transaction, &mut text);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_journal;

    #[test]
    fn test_format_journal() {
        let text = include_str!("../test_data/simple.sou");

        let journal = parse_journal(text).expect("Stuff");

        let text = format_journal(&journal);

        assert_eq!(journal, parse_journal(&text).expect("Stuff"));
    }
}
