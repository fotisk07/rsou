use crate::queries::{BsReport, LedgerEntry, PnlReport};
use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Style, object::Columns},
};

pub fn format_ledger(account: &str, entries: &[LedgerEntry]) -> String {
    if entries.is_empty() {
        return format!("Ledger — {account}\n\nNo transactions.\nClosing balance: 0.00\n");
    }

    let mut builder = Builder::default();
    builder.push_record(["Date", "Account", "Description", "Amount", "Balance"]);

    for entry in entries {
        builder.push_record([
            entry.date.to_string(),
            entry.account.clone(),
            entry.description.clone(),
            format!("{:.2}", entry.amount),
            format!("{:.2}", entry.balance),
        ]);
    }

    let closing = entries.last().unwrap().balance; // Entries are nonempty here.
    builder.push_record([
        String::new(),
        String::new(),
        "Closing balance".to_string(),
        String::new(),
        format!("{closing:.2}"),
    ]);

    let mut table = builder.build();
    table.with(Style::blank());
    table.with(Modify::new(Columns::new(3..5)).with(Alignment::right()));

    format!("Ledger — {account}\n\n{table}\n")
}

pub fn format_pnl(report: &PnlReport) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Account", "Amount"]);
    for row in &report.rows {
        builder.push_record([
            format!("{}{}", "  ".repeat(row.depth), row.account),
            format!("{:.2}", row.amount),
        ]);
    }
    builder.push_record([String::new(), String::new()]);
    builder.push_record([
        "Net Result".to_string(),
        format!("{:.2}", report.net_result),
    ]);

    let mut table = builder.build();
    table.with(Style::blank());
    table.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
    format!("Profit and loss\n\n{table}\n")
}

pub fn format_bs(report: &BsReport) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Account", "Amount"]);
    for row in &report.rows {
        builder.push_record([
            format!("{}{}", "  ".repeat(row.depth), row.account),
            format!("{:.2}", row.amount),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::blank());
    table.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
    format!("Balance sheet\n\n{table}\n")
}

#[cfg(test)]
mod tests {
    use super::format_ledger;
    use crate::{parser::parse_journal, queries::ledger_entries};

    #[test]
    fn formats_cash_ledger() {
        let journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();
        let entries = ledger_entries(&journal, "Assets::Cash").unwrap();
        let text = format_ledger("Assets::Cash", &entries);

        assert!(text.starts_with("Ledger — Assets::Cash\n"));
        assert!(text.contains("2026-02-03"));
        assert!(text.contains("Assets::Cash::Checking"));
        assert!(text.contains("Groceries and snacks"));
        assert!(text.contains("-23.75"));
        assert!(text.contains("176.25"));
        assert!(text.contains("181.25"));
        assert!(text.contains("Closing balance"));
    }

    #[test]
    fn formats_empty_ledger() {
        assert_eq!(
            format_ledger("Income", &[]),
            "Ledger — Income\n\nNo transactions.\nClosing balance: 0.00\n"
        );
    }
}
