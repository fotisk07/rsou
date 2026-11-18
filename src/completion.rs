use crate::{load_journal, models::Account};
use clap_complete::CompletionCandidate;
use std::ffi::{OsStr, OsString};

pub fn accounts(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(prefix) = current.to_str() else {
        return Vec::new();
    };
    let path = journal_path();
    let Some(path) = path.to_str() else {
        return Vec::new();
    };
    let Ok(journal) = load_journal(path) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();
    for root in ["Assets", "Liabilities", "Equity", "Income", "Expenses"] {
        let account = journal
            .accounts
            .find_account(root)
            .expect("built-in root account");
        collect_accounts(account, root.to_string(), prefix, &mut candidates);
    }
    candidates
}

fn collect_accounts(
    account: &Account,
    path: String,
    prefix: &str,
    candidates: &mut Vec<CompletionCandidate>,
) {
    if path.starts_with(prefix) {
        candidates.push(CompletionCandidate::new(path.clone()));
    }
    for child in &account.children {
        collect_accounts(child, format!("{path}/{}", child.name), prefix, candidates);
    }
}

// CompleteEnv passes the shell's command words after `--`. Only inspect words
// before the cursor, so a later journal option cannot affect earlier completion.
fn journal_path() -> OsString {
    let cursor = std::env::var("_CLAP_COMPLETE_INDEX")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(usize::MAX);
    let mut words = std::env::args_os()
        .skip_while(|word| word != "--")
        .skip(1)
        .take(cursor);
    let mut path = OsString::from("journal.sou");
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        if word == "-j" || word == "--journal-path" {
            if let Some(value) = words.next() {
                path = value;
            }
        } else if let Some(word) = word.to_str() {
            if let Some(value) = word
                .strip_prefix("--journal-path=")
                .or_else(|| word.strip_prefix("-j").filter(|value| !value.is_empty()))
            {
                path = value.into();
            }
        }
    }
    path
}
