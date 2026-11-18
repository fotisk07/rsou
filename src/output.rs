use crate::queries::{BsReport, LedgerEntry, PnlReport};
use rust_decimal::Decimal;
use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Style, object::Columns, style::HorizontalLine},
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
            format_report_amount(entry.amount),
            format_report_amount(entry.balance),
        ]);
    }

    let closing = entries.last().unwrap().balance; // Entries are nonempty here.
    let closing_row = builder.count_records();
    builder.push_record([
        String::new(),
        String::new(),
        "Closing balance".to_string(),
        String::new(),
        format_report_amount(closing),
    ]);

    let mut table = builder.build();
    table.with(Style::blank().horizontals([
        (1, HorizontalLine::new('─').intersection('─')),
        (closing_row, HorizontalLine::new('─').intersection('─')),
    ]));
    table.with(Modify::new(Columns::new(3..5)).with(Alignment::right()));

    format!("Ledger — {account}\n\n{table}\n")
}

pub fn format_pnl(report: &PnlReport) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Account", "Amount"]);
    let expenses_start = report
        .rows
        .iter()
        .position(|row| row.depth == 0 && row.account == "Expenses")
        .unwrap_or(report.rows.len());
    let (income, expenses) = report.rows.split_at(expenses_start);

    for (name, rows) in [("Income", income), ("Expenses", expenses)] {
        if name == "Expenses" {
            builder.push_record(["", ""]);
        }
        builder.push_record([name.to_uppercase(), String::new()]);
        let mut total = Decimal::ZERO;
        for row in rows {
            if row.depth == 0 {
                total = row.amount;
            } else {
                builder.push_record([
                    format!("{}{}", "  ".repeat(row.depth), row.account),
                    format_report_amount(row.amount),
                ]);
            }
        }
        builder.push_record([format!("Total {name}"), format_report_amount(total)]);
    }
    let result_row = builder.count_records();
    builder.push_record([
        "Net Result".to_string(),
        format_report_amount(report.net_result),
    ]);

    let mut table = builder.build();
    table.with(Style::blank().horizontals([
        (1, HorizontalLine::new('─').intersection('─')),
        (result_row, HorizontalLine::new('─').intersection('─')),
    ]));
    table.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
    format!("Profit and Loss\n\n{table}\n")
}

fn format_report_amount(amount: Decimal) -> String {
    let text = format!("{:.2}", amount.abs());
    let (integer, fraction) = text.split_once('.').expect("fixed two-decimal amount");
    let mut grouped = String::new();
    if amount < Decimal::ZERO {
        grouped.push('-');
    }
    for (index, digit) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    format!("{grouped}.{fraction}")
}

pub fn format_bs(report: &BsReport) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Account", "Amount"]);
    let mut rows = report.rows.iter().peekable();
    let mut liabilities_and_equity = Decimal::ZERO;
    while let Some(root) = rows.next() {
        if root.account != "Assets" {
            builder.push_record(["", ""]);
            liabilities_and_equity += root.amount;
        }
        builder.push_record([root.account.to_uppercase(), String::new()]);
        while rows.peek().is_some_and(|row| row.depth > 0) {
            let row = rows.next().expect("peek found a detail row");
            builder.push_record([
                format!("{}{}", "  ".repeat(row.depth), row.account),
                format_report_amount(row.amount),
            ]);
        }
        builder.push_record([
            format!("Total {}", root.account),
            format_report_amount(root.amount),
        ]);
    }
    let total_row = builder.count_records();
    builder.push_record([
        "Total Liabilities and Equity".to_string(),
        format_report_amount(liabilities_and_equity),
    ]);

    let mut table = builder.build();
    table.with(Style::blank().horizontals([
        (1, HorizontalLine::new('─').intersection('─')),
        (total_row, HorizontalLine::new('─').intersection('─')),
    ]));
    table.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
    format!("Balance Sheet\n\n{table}\n")
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
