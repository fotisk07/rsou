use crate::models::{Account, Accounts, Journal, Transaction};
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub fn format_accounts(accounts: &Accounts) -> String {
    let mut text = String::new();
    for root_name in ["Assets", "Liabilities", "Equity", "Income", "Expenses"] {
        let root = accounts
            .find_account(root_name)
            .expect("root accounts always exist");
        format_account(root, 0, &mut text);
    }
    text
}

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

fn format_journal(journal: &Journal) -> String {
    // Metadata
    let mut text = format!(
        "[JOURNAL]\n\nname: {}\nyear: {}\ncurrency: {}\n\n",
        journal.metadata.name, journal.metadata.year, journal.metadata.currency,
    );

    // Accounts
    text.push_str("[ACCOUNTS]\n\n");
    text.push_str(&format_accounts(&journal.accounts));

    if !journal.aliases.is_empty() {
        text.push_str("\n[ALIASES]\n\n");
        for alias in &journal.aliases {
            text.push_str(&format!("{} = {}\n", alias.name, alias.account));
        }
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
pub fn write_journal(path: String, journal: &Journal) -> io::Result<()> {
    let file = journal_file(Path::new(&path), journal)?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

pub fn create_journal(path: String, journal: &Journal) -> io::Result<()> {
    let file = journal_file(Path::new(&path), journal)?;
    file.persist_noclobber(path).map_err(|error| error.error)?;
    Ok(())
}

fn journal_file(path: &Path, journal: &Journal) -> io::Result<NamedTempFile> {
    let journal_text = format_journal(journal);
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let mut file = NamedTempFile::new_in(directory)?;
    file.write_all(journal_text.as_bytes())?;
    file.as_file().sync_all()?;
    Ok(file)
}
