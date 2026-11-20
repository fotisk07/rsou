use crate::models::{Journal, JournalMetadata, Posting, RecurringTransaction, Transaction};
use jiff::civil::Date;
use rust_decimal::Decimal;

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    UnknownAccount(String),
    InvalidAccountName(String),
    InvalidAlias(String),
    DuplicateAlias(String),
    AliasConflict(String),
    Unbalanced(String),
    NotEnoughPostings,
    InvalidDescription,
    NextDateBeforeStart,
    TooManyDecimalPlaces(String),
    DateOutsideJournalYear { date: Date, year: i32 },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DateOutsideJournalYear { date, year } => {
                write!(f, "transaction date {date} is outside journal year {year}")
            }
            Self::NextDateBeforeStart => {
                write!(f, "recurring next date cannot precede its start date")
            }
            Self::UnknownAccount(account) => write!(f, "unknown account `{account}`"),
            Self::InvalidAlias(name) => write!(f, "invalid alias `{name}`"),
            Self::DuplicateAlias(name) => write!(f, "duplicate alias `{name}`"),
            Self::AliasConflict(name) => write!(f, "alias `{name}` conflicts with an account name"),
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
    validate_postings(journal, &transaction.description, &transaction.postings)
}

pub fn validate_recurring(
    journal: &Journal,
    recurring: &RecurringTransaction,
) -> Result<(), ValidationError> {
    if i32::from(recurring.start_date.year()) != journal.metadata.year {
        return Err(ValidationError::DateOutsideJournalYear {
            date: recurring.start_date,
            year: journal.metadata.year,
        });
    }
    if recurring.next_date < recurring.start_date {
        return Err(ValidationError::NextDateBeforeStart);
    }
    validate_postings(journal, &recurring.description, &recurring.postings)
}

fn validate_postings(
    journal: &Journal,
    description: &str,
    postings: &[Posting],
) -> Result<(), ValidationError> {
    if description.trim().is_empty() || description.contains(['\n', '\r']) {
        return Err(ValidationError::InvalidDescription);
    }
    let mut balance = Decimal::ZERO;
    if postings.len() < 2 {
        return Err(ValidationError::NotEnoughPostings);
    }
    for posting in postings {
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
        return Err(ValidationError::Unbalanced(description.to_string()));
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
    if let Some(author) = &metadata.author
        && (author.trim().is_empty()
            || author
                .chars()
                .any(|c| c.is_control() || matches!(c, '[' | ']')))
    {
        return Err(
            "journal author must be nonempty and contain no control characters or brackets"
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
    let mut accounts: Vec<_> = journal.accounts.roots().collect();
    while let Some(account) = accounts.pop() {
        validate_account_name(&account.name)?;
        accounts.extend(&account.children);
    }

    for (index, alias) in journal.aliases.iter().enumerate() {
        if validate_account_name(&alias.name).is_err()
            || alias.name.starts_with('-')
            || alias.name.contains('=')
        {
            return Err(ValidationError::InvalidAlias(alias.name.clone()));
        }
        if journal.accounts.find_account(&alias.name).is_some() {
            return Err(ValidationError::AliasConflict(alias.name.clone()));
        }
        if journal.aliases[..index]
            .iter()
            .any(|other| other.name == alias.name)
        {
            return Err(ValidationError::DuplicateAlias(alias.name.clone()));
        }
        if journal.accounts.find_account(&alias.account).is_none() {
            return Err(ValidationError::UnknownAccount(alias.account.clone()));
        }
    }

    for transaction in &journal.transactions {
        validate_transaction(journal, transaction)?;
    }
    for recurring in &journal.recurring_transactions {
        validate_recurring(journal, recurring)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::parse_journal;

    #[test]
    fn transactions_require_balanced_postings_to_known_accounts_in_whole_cents() {
        let journal = parse_journal(
            "[JOURNAL]\nname: Test\nauthor:\nyear: 2024\ncurrency: EUR\n\
             [ACCOUNTS]\nAssets\nExpenses\n[TRANSACTIONS]\n",
        )
        .unwrap();
        let mut transaction = Transaction::new(
            "2024-02-29".parse().unwrap(),
            "Split purchase".into(),
            vec![
                Posting {
                    account: "Assets".into(),
                    amount: Decimal::new(-1234, 2),
                },
                Posting {
                    account: "Expenses".into(),
                    amount: Decimal::new(1000, 2),
                },
                Posting {
                    account: "Expenses".into(),
                    amount: Decimal::new(234, 2),
                },
            ],
        );
        assert_eq!(validate_transaction(&journal, &transaction), Ok(()));

        transaction.postings[2].amount = Decimal::new(235, 2);
        assert!(matches!(
            validate_transaction(&journal, &transaction),
            Err(ValidationError::Unbalanced(_))
        ));
        transaction.postings[2].amount = Decimal::new(234, 2);

        transaction.postings[2].account = "Expenses/Missing".into();
        assert!(matches!(
            validate_transaction(&journal, &transaction),
            Err(ValidationError::UnknownAccount(_))
        ));
        transaction.postings[2].account = "Expenses".into();

        transaction.postings[0].amount = Decimal::new(-12345, 3);
        transaction.postings[2].amount = Decimal::new(2345, 3);
        assert!(matches!(
            validate_transaction(&journal, &transaction),
            Err(ValidationError::TooManyDecimalPlaces(_))
        ));
    }
}
