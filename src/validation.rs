use crate::models::{Accounts, Journal, Transaction};
use rust_decimal::Decimal;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    UnknownAccount(String),
    Unbalanced(String),
    NotEnoughPostings,
    InvalidDescription,
    TooManyDecimalPlaces(String),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::UnknownAccount(account) => write!(f, "unknown account `{account}`"),
            ValidationError::Unbalanced(description) => {
                write!(f, "unbalanced transaction `{description}`")
            }
            ValidationError::NotEnoughPostings => {
                write!(f, "transaction needs at least two postings")
            }
            ValidationError::InvalidDescription => {
                write!(f, "description must be nonempty and contain no line breaks")
            }
            ValidationError::TooManyDecimalPlaces(account) => write!(
                f,
                "amount for `{account}` requires more than two decimal places"
            ),
        }
    }
}

pub fn validate_transaction(
    accounts: &Accounts,
    transaction: &Transaction,
) -> Result<(), ValidationError> {
    if transaction.description.trim().is_empty() || transaction.description.contains(['\n', '\r']) {
        return Err(ValidationError::InvalidDescription);
    }
    let mut balance = Decimal::ZERO;
    if transaction.postings.len() < 2 {
        return Err(ValidationError::NotEnoughPostings);
    }
    for posting in &transaction.postings {
        if posting.amount.normalize().scale() > 2 {
            return Err(ValidationError::TooManyDecimalPlaces(
                posting.account.clone(),
            ));
        }
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
    use super::{ValidationError, validate_journal};
    use crate::parser::parse_journal;

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

    #[test]
    fn errors_read_as_sentences() {
        assert_eq!(
            ValidationError::UnknownAccount("Expenses::Missing".to_string()).to_string(),
            "unknown account `Expenses::Missing`"
        );
        assert_eq!(
            ValidationError::NotEnoughPostings.to_string(),
            "transaction needs at least two postings"
        );
    }
}
