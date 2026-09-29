use crate::models::{Accounts, Journal, Transaction};
use rust_decimal::Decimal;

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    UnknownAccount(String),
    Unbalanced(String),
    NotEnoughPostings,
}

pub fn validate_transaction(
    accounts: &Accounts,
    transaction: &Transaction,
) -> Result<(), ValidationError> {
    let mut balance = Decimal::ZERO;
    if transaction.postings.len() < 2 {
        return Err(ValidationError::NotEnoughPostings);
    }
    for posting in &transaction.postings {
        if accounts.find_account(&posting.account).is_none() {
            return Err(ValidationError::UnknownAccount(posting.account.clone()));
        }
        balance += posting.amount;
    }
    if balance != Decimal::ZERO {
        return Err(ValidationError::Unbalanced(transaction.description.clone()));
    }
    Ok(())
}

pub fn validate_journal(journal: &Journal) -> Result<(), ValidationError> {
    for transaction in &journal.transactions {
        validate_transaction(&journal.accounts, transaction)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ValidationError, validate_journal, validate_transaction};
    use crate::parser::parse_journal;
    use rust_decimal::Decimal;

    #[test]
    fn valid_transaction() {
        let journal = parse_journal(include_str!("../test_data/simple.sou")).unwrap();
        assert_eq!(
            validate_transaction(&journal.accounts, &journal.transactions[1]),
            Ok(())
        );
    }

    #[test]
    fn unbalanced_transaction() {
        let mut journal = parse_journal(include_str!("../test_data/simple.sou")).unwrap();
        journal.transactions[1].postings[1].amount = Decimal::from(5);
        assert_eq!(
            validate_transaction(&journal.accounts, &journal.transactions[1]),
            Err(ValidationError::Unbalanced("Coffee and lunch".to_string()))
        );
    }

    #[test]
    fn valid_parsed_journal() {
        let journal = parse_journal(include_str!("../test_data/simple.sou")).expect("Stuff");
        assert_eq!(validate_journal(&journal), Ok(()));
    }

    #[test]
    fn parsed_journal_with_unknown_account() {
        let text = include_str!("../test_data/simple.sou")
            .replace("Expenses::Food  12.50", "Expenses::Missing  12.50");
        let journal = parse_journal(&text).expect("Stuff");
        assert_eq!(
            validate_journal(&journal),
            Err(ValidationError::UnknownAccount(
                "Expenses::Missing".to_string()
            ))
        );
    }

    #[test]
    fn parsed_journal_with_unbalanced_transaction() {
        let text = include_str!("../test_data/simple.sou")
            .replace("Expenses::Food  12.50", "Expenses::Food  10.00");
        let journal = parse_journal(&text).expect("Stuff");
        assert_eq!(
            validate_journal(&journal),
            Err(ValidationError::Unbalanced("Coffee and lunch".to_string()))
        );
    }

    #[test]
    fn parsed_journal_with_insufficient_transactions() {
        let text = include_str!("../test_data/simple.sou").replace("Expenses::Food  12.50", "");
        let journal = parse_journal(&text).unwrap();
        assert_eq!(
            validate_journal(&journal),
            Err(ValidationError::NotEnoughPostings)
        );
    }
}
