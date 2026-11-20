use crate::{load_journal, models::Account};
use clap_complete::CompletionCandidate;
use std::ffi::{OsStr, OsString};

pub fn accounts(current: &OsStr) -> Vec<CompletionCandidate> {
    posting_accounts(current)
}

pub fn posting_accounts(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_accounts(current, false, true)
}

pub fn account_parents(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_accounts(current, true, false)
}

fn complete_accounts(
    current: &OsStr,
    adding: bool,
    include_aliases: bool,
) -> Vec<CompletionCandidate> {
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

    let (base, partial) = split_path(prefix);
    if base.is_empty() {
        let roots = ["Assets", "Liabilities", "Equity", "Income", "Expenses"]
            .into_iter()
            .filter_map(|root| journal.accounts.find_account(root))
            .filter_map(|account| complete_component(account, base, partial, adding));
        let aliases = journal
            .aliases
            .iter()
            .filter(|alias| include_aliases && alias.name.starts_with(partial))
            .map(|alias| CompletionCandidate::new(&alias.name));
        return roots.chain(aliases).collect();
    }
    let Some(parent) = journal.accounts.find_account(base.trim_end_matches('/')) else {
        return Vec::new();
    };
    parent
        .children
        .iter()
        .filter_map(|account| complete_component(account, base, partial, adding))
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
fn journal_path() -> OsString {
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
    while let Some(word) = words.next() {
        if word == "--" {
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
        }
    }
    path
}
