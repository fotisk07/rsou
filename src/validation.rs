use crate::models::{Journal, JournalMetadata, Transaction};
use jiff::civil::Date;
use rust_decimal::Decimal;

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    UnknownAccount(String),
    InvalidAccountName(String),
    Unbalanced(String),
    NotEnoughPostings,
    InvalidDescription,
    TooManyDecimalPlaces(String),
    DateOutsideJournalYear { date: Date, year: i32 },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DateOutsideJournalYear { date, year } => {
                write!(f, "transaction date {date} is outside journal year {year}")
            }
            Self::UnknownAccount(account) => write!(f, "unknown account `{account}`"),
            Self::InvalidAccountName(name) => write!(
                f,
                "invalid account name `{name}`: names must be nonempty and contain no whitespace, slashes, colons, or brackets"
            ),
            Self::Unbalanced(description) => {
                write!(f, "unbalanced transaction `{description}`")
            }
            Self::NotEnoughPostings => write!(f, "transaction needs at least two postings"),
            Self::InvalidDescription => {
                write!(f, "description must be nonempty and contain no line breaks")
            }
            Self::TooManyDecimalPlaces(account) => {
                write!(
                    f,
                    "amount for `{account}` requires more than two decimal places"
                )
            }
        }
    }
}

pub fn validate_transaction(
    journal: &Journal,
    transaction: &Transaction,
) -> Result<(), ValidationError> {
    if i32::from(transaction.date.year()) != journal.metadata.year {
        return Err(ValidationError::DateOutsideJournalYear {
            date: transaction.date,
            year: journal.metadata.year,
        });
    }
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
        if journal.accounts.find_account(&posting.account).is_none() {
            return Err(ValidationError::UnknownAccount(posting.account.clone()));
        }
        balance += posting.amount;
    }
    if balance != Decimal::ZERO {
        return Err(ValidationError::Unbalanced(transaction.description.clone()));
    }
    Ok(())
}

pub fn validate_metadata(metadata: &JournalMetadata) -> Result<(), String> {
    if metadata.name.trim().is_empty()
        || metadata
            .name
            .chars()
            .any(|c| c.is_control() || matches!(c, '[' | ']'))
    {
        return Err(
            "journal name must be nonempty and contain no control characters or brackets"
                .to_string(),
        );
    }
    if !(1..=9999).contains(&metadata.year) {
        return Err("year must be between 1 and 9999".to_string());
    }
    if metadata.currency.len() != 3
        || !metadata
            .currency
            .bytes()
            .all(|byte| byte.is_ascii_uppercase())
    {
        return Err("currency must be three uppercase ASCII letters".to_string());
    }
    Ok(())
}

pub fn validate_account_path(path: &str) -> Result<(), ValidationError> {
    path.split('/').try_for_each(validate_account_name)
}

fn validate_account_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '/' | ':' | '[' | ']'))
    {
        return Err(ValidationError::InvalidAccountName(name.to_string()));
    }
    Ok(())
}

pub fn validate_journal(journal: &Journal) -> Result<(), ValidationError> {
    let mut accounts: Vec<_> = ["Assets", "Liabilities", "Equity", "Income", "Expenses"]
        .iter()
        .map(|root| {
            journal
                .accounts
                .find_account(root)
                .expect("built-in root account")
        })
        .collect();
    while let Some(account) = accounts.pop() {
        validate_account_name(&account.name)?;
        accounts.extend(&account.children);
    }

    for transaction in &journal.transactions {
        validate_transaction(journal, transaction)?;
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
            .replace("Expenses/Food  12.50", "Expenses/Missing  12.50");
        let journal = parse_journal(&text).expect("Stuff");
        assert_eq!(
            validate_journal(&journal),
            Err(ValidationError::UnknownAccount(
                "Expenses/Missing".to_string()
            ))
        );
    }

    #[test]
    fn parsed_journal_with_unbalanced_transaction() {
        let text = include_str!("../test_data/simple.sou")
            .replace("Expenses/Food  12.50", "Expenses/Food  10.00");
        let journal = parse_journal(&text).expect("Stuff");
        assert_eq!(
            validate_journal(&journal),
            Err(ValidationError::Unbalanced("Coffee and lunch".to_string()))
        );
    }

    #[test]
    fn parsed_journal_with_insufficient_transactions() {
        let text = include_str!("../test_data/simple.sou").replace("Expenses/Food  12.50", "");
        let journal = parse_journal(&text).unwrap();
        assert_eq!(
            validate_journal(&journal),
            Err(ValidationError::NotEnoughPostings)
        );
    }
}
