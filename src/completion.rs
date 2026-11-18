use crate::{cli::AccountType, load_journal, models::Account};
use clap::ValueEnum;
use clap_complete::CompletionCandidate;
use std::ffi::{OsStr, OsString};

pub fn accounts(current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(prefix) = current.to_str() else {
        return Vec::new();
    };
    let (path, _) = completion_context();
    let Some(path) = path.to_str() else {
        return Vec::new();
    };
    let Ok(journal) = load_journal(path) else {
        return Vec::new();
    };

    let (base, partial) = split_path(prefix);
    if base.is_empty() {
        return ["Assets", "Liabilities", "Equity", "Income", "Expenses"]
            .into_iter()
            .filter_map(|root| journal.accounts.find_account(root))
            .filter_map(|account| complete_component(account, base, partial, false))
            .collect();
    }
    let Some(parent) = journal.accounts.find_account(base.trim_end_matches('/')) else {
        return Vec::new();
    };
    parent
        .children
        .iter()
        .filter_map(|account| complete_component(account, base, partial, false))
        .collect()
}

pub fn account_parents(current: &OsStr) -> Vec<CompletionCandidate> {
    let (path, positionals) = completion_context();
    let [command, account_type] = positionals.as_slice() else {
        return Vec::new();
    };
    if command != "add" {
        return Vec::new();
    }
    let (Some(prefix), Some(path), Some(account_type)) =
        (current.to_str(), path.to_str(), account_type.to_str())
    else {
        return Vec::new();
    };
    let Ok(account_type) = AccountType::from_str(account_type, false) else {
        return Vec::new();
    };
    let Ok(journal) = load_journal(path) else {
        return Vec::new();
    };

    let root = account_type.root_name();
    let (base, partial) = split_path(prefix);
    let parent_path = if base.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{}", base.trim_end_matches('/'))
    };
    let Some(account) = journal.accounts.find_account(&parent_path) else {
        return Vec::new();
    };
    account
        .children
        .iter()
        .filter_map(|child| complete_component(child, base, partial, true))
        .collect()
}

fn split_path(prefix: &str) -> (&str, &str) {
    let end = prefix.rfind('/').map_or(0, |index| index + 1);
    prefix.split_at(end)
}

fn complete_component(
    account: &Account,
    base: &str,
    partial: &str,
    adding: bool,
) -> Option<CompletionCandidate> {
    if !account.name.starts_with(partial) {
        return None;
    }
    let suffix = if adding || !account.children.is_empty() {
        "/"
    } else {
        ""
    };
    Some(CompletionCandidate::new(format!(
        "{base}{}{suffix}",
        account.name
    )))
}

// CompleteEnv passes the shell's command words after `--`. Only inspect words
// before the cursor, so a later journal option cannot affect earlier completion.
fn completion_context() -> (OsString, Vec<OsString>) {
    let cursor = std::env::var("_CLAP_COMPLETE_INDEX")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(usize::MAX);
    let mut words = std::env::args_os()
        .skip_while(|word| word != "--")
        .skip(1)
        .take(cursor)
        .skip(1);
    let mut path = OsString::from("journal.sou");
    let mut positionals = Vec::new();
    while let Some(word) = words.next() {
        if word == "--" {
            positionals.extend(words);
            break;
        }
        if word == "-j" || word == "--journal-path" {
            if let Some(value) = words.next() {
                path = value;
            }
        } else if let Some(word) = word.to_str()
            && let Some(value) = word
                .strip_prefix("--journal-path=")
                .or_else(|| word.strip_prefix("-j").filter(|value| !value.is_empty()))
        {
            path = value.into();
        } else {
            positionals.push(word);
        }
    }
    (path, positionals)
}
