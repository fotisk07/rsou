use crate::models::{Account, Accounts, Journal, Posting, Transaction};
use chrono::NaiveDate;
use regex::Regex;
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn parse_journal(journal_text: &str) -> Journal {
    let accounts_section_re = Regex::new(r"(?s)\[ACCOUNTS\]\s*(.*?)\s*\[TRANSACTIONS\]").unwrap();

    let accounts_text = accounts_section_re
        .captures(journal_text)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();
    let transactions_text = journal_text
        .split_once("[TRANSACTIONS]")
        .map(|(_, rest)| rest.trim())
        .unwrap();

    let accounts = parse_accounts(accounts_text.to_string());
    let transactions = parse_transactions(transactions_text.to_string());

    Journal {
        accounts,
        transactions,
    }
}

fn parse_accounts(accounts_text: String) -> Accounts {
    let mut accounts = Accounts::new();
    let mut stack: Vec<String> = Vec::new();

    for line in accounts_text.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let depth = line.chars().take_while(|c| *c == ' ').count() / 2;
        let name = line.trim().to_string();

        stack.truncate(depth);
        stack.push(name.clone());

        if depth == 0 {
            accounts.root_mut(&name).expect("Invalid root account");
            continue;
        }

        let root_name = &stack[0];
        let mut parent = accounts.root_mut(root_name).expect("Unknown root account");

        for parent_name in &stack[1..depth] {
            parent = parent
                .children
                .iter_mut()
                .find(|account| account.name == *parent_name)
                .expect("Parent account not found");
        }

        parent.children.push(Account {
            name,
            children: vec![],
        });
    }
    accounts
}

fn parse_transactions(transactions_text: String) -> Vec<Transaction> {
    let mut lines = transactions_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .peekable();

    let mut transactions: Vec<Transaction> = vec![];

    while let Some(header) = lines.next() {
        let (date_str, description) = header
            .split_once(char::is_whitespace)
            .expect("invalid transaction header");

        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();

        let mut postings: Vec<Posting> = vec![];

        while lines.peek().is_some_and(|l| l.starts_with(" ")) {
            let posting_line = lines.next().unwrap();
            let mut posting_parts = posting_line.split_whitespace();
            let account_str = posting_parts.next().unwrap();
            let amount_str = posting_parts.next().unwrap();

            postings.push(Posting {
                account: account_str.to_string(),
                amount: Decimal::from_str(amount_str).unwrap(),
            });
        }
        transactions.push(Transaction {
            date,
            description: description.to_string(),
            postings,
        });
    }
    transactions
}

#[cfg(test)]
mod tests {
    use super::parse_journal;

    #[test]
    fn parses_simple_journal() {
        let text = include_str!("../test_data/simple.sou");
        let journal = parse_journal(text);

        assert_eq!(journal.transactions.len(), 2);
        assert_eq!(journal.transactions[1].description, "Coffee and lunch");
    }

    #[test]
    fn parses_split_journal() {
        let text = include_str!("../test_data/split.sou");
        let journal = parse_journal(text);

        assert_eq!(journal.transactions.len(), 3);
        assert_eq!(journal.transactions[1].postings.len(), 3);
    }

    #[test]
    #[should_panic]
    fn parses_failed_journal() {
        let text = include_str!("../test_data/invalid-date.sou");
        parse_journal(text);
    }
}
