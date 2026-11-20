use super::{Report, ReportRow, RowKind};
use crate::dates::DateRange;
use crate::models::{Account, Accounts, RecurringTransaction};
use crate::queries::LedgerReport;
use jiff::civil::Date;
use rust_decimal::Decimal;
use tabled::{
    builder::Builder,
    settings::{
        Alignment, Modify, Style,
        object::Columns,
        style::{HorizontalLine, VerticalLine},
    },
};

pub fn format_accounts(accounts: &Accounts) -> String {
    fn add_account(account: &Account, depth: usize, text: &mut String) {
        text.push_str(&"  ".repeat(depth));
        text.push_str(&account.name);
        text.push('\n');
        for child in &account.children {
            add_account(child, depth + 1, text);
        }
    }

    let mut text = String::new();
    for root in accounts.roots() {
        add_account(root, 0, &mut text);
    }
    text
}

pub fn format_recurring(templates: &[RecurringTransaction], details: bool) -> String {
    if templates.is_empty() {
        return "No recurring transactions.\n".to_string();
    }
    if details {
        let mut text = String::new();
        for (index, template) in templates.iter().enumerate() {
            if index > 0 {
                text.push('\n');
            }
            text.push_str(&format!(
                "{} · {} · next: {}\n  Start: {}\n",
                template.description, template.frequency, template.next_date, template.start_date,
            ));
            let mut builder = Builder::default();
            for posting in &template.postings {
                builder.push_record([
                    format!(" {}", posting.account),
                    format_report_amount(posting.amount),
                ]);
            }
            let mut table = builder.build();
            table.with(Style::blank());
            table.with(Modify::new(Columns::new(1..2)).with(Alignment::right()));
            text.push_str(&format!("{table}\n"));
        }
        return text;
    }
    let mut builder = Builder::default();
    builder.push_record(["Description", "Frequency", "Next"]);
    for template in templates {
        builder.push_record([
            template.description.clone(),
            template.frequency.to_string(),
            template.next_date.to_string(),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::blank().horizontals([(1, HorizontalLine::new('─').intersection('─'))]));
    format!("{table}\n")
}

pub fn format_ledger(account: &str, report: &LedgerReport, year: i32) -> String {
    let heading = format!("Ledger | {account} | {}", format_period(report.range, year));
    if report.entries.is_empty() && report.range.from.is_none() {
        return format!("{heading}\n\nNo transactions.\nClosing balance: 0.00\n");
    }
    let mut builder = Builder::default();
    builder.push_record(["Date", "Account", "Description", "Amount", "Balance"]);
    if report.range.from.is_some() {
        builder.push_record([
            String::new(),
            String::new(),
            "Opening balance".to_string(),
            String::new(),
            format_report_amount(report.opening),
        ]);
    }
    for entry in &report.entries {
        builder.push_record([
            entry.date.to_string(),
            entry.account.clone(),
            entry.description.clone(),
            format_report_amount(entry.amount),
            format_report_amount(entry.balance),
        ]);
    }
    let closing = report
        .entries
        .last()
        .map_or(report.opening, |entry| entry.balance);
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
    format!("{heading}\n\n{table}\n")
}

pub(super) fn report(report: &Report, depth: usize) -> String {
    let mut builder = Builder::default();
    let mut header = vec!["Account".to_string()];
    header.extend(report.columns.iter().cloned());
    builder.push_record(header);
    let mut root: Option<&ReportRow> = None;
    let mut result_row = 0;
    for row in &report.rows {
        if row.depth == 0 {
            if let Some(previous) = root.take() {
                add_row(
                    &mut builder,
                    format!("Total {}", previous.name),
                    &previous.amounts,
                );
            }
            if row.kind == RowKind::Result {
                result_row = builder.count_records();
                add_row(&mut builder, row.name.clone(), &row.amounts);
            } else {
                if builder.count_records() > 1 {
                    builder.push_record(vec![String::new(); report.columns.len() + 1]);
                }
                if depth > 0 {
                    let mut heading = vec![String::new(); report.columns.len() + 1];
                    heading[0] = row.name.to_uppercase();
                    builder.push_record(heading);
                }
                root = Some(row);
            }
        } else {
            add_row(
                &mut builder,
                format!("{}{}", "  ".repeat(row.depth), row.name),
                &row.amounts,
            );
        }
    }
    let mut table = builder.build();
    let style = Style::blank().horizontals([
        (1, HorizontalLine::new('─').intersection('─')),
        (result_row, HorizontalLine::new('─').intersection('─')),
    ]);
    if report.has_total_column {
        table.with(style.verticals([(report.columns.len(), VerticalLine::new('│'))]));
    } else {
        table.with(style);
    }
    table.with(Modify::new(Columns::new(1..report.columns.len() + 1)).with(Alignment::right()));
    format!("{} | {}\n\n{table}\n", report.title, report.period)
}

fn add_row(builder: &mut Builder, name: String, amounts: &[Decimal]) {
    let mut record = vec![name];
    record.extend(amounts.iter().map(|amount| format_report_amount(*amount)));
    builder.push_record(record);
}

pub(super) fn format_period(range: DateRange, year: i32) -> String {
    let from = range
        .from
        .map(format_date)
        .unwrap_or_else(|| format!("01 Jan {year:04}"));
    let to = range
        .to
        .map(format_date)
        .unwrap_or_else(|| format!("31 Dec {year:04}"));
    format!("{from} to {to}")
}

fn format_date(date: Date) -> String {
    date.strftime("%d %b %Y").to_string()
}

pub(super) fn format_report_amount(amount: Decimal) -> String {
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
