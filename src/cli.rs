use crate::models::Journal;
use crate::parser::parse_journal;
use crate::validation::{ValidationError, validate_journal};
use std::fs;

/// Read, parse, and validate a journal without printing or exiting.
pub fn load_journal(path: &str) -> Result<Journal, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;

    let journal = parse_journal(&contents).map_err(|error| match error.line {
        Some(line) => format!("parse error (line {line}): {}", error.message),
        None => format!("parse error: {}", error.message),
    })?;

    validate_journal(&journal).map_err(|error| match error {
        ValidationError::UnknownAccount(account) => {
            format!("validation error: unknown account `{account}`")
        }
        ValidationError::Unbalanced(description) => {
            format!("validation error: unbalanced transaction `{description}`")
        }
        ValidationError::NotEnoughPostings => {
            "validation error: transaction needs at least two postings".to_string()
        }
    })?;

    Ok(journal)
}
