use crate::dates::DateRange;
use crate::models::RecurringTransaction;
use crate::queries::{BsReport, LedgerReport, PnlReport};
use jiff::civil::Date;
use rust_decimal::Decimal;
use tabled::{
    builder::Builder,
    settings::{Alignment, Modify, Style, object::Columns, style::HorizontalLine},
};

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

pub fn format_pnl(report: &PnlReport, year: i32, depth: usize) -> String {
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
        if depth > 0 {
            builder.push_record([name.to_uppercase(), String::new()]);
        }
        let mut total = Decimal::ZERO;
        for (index, row) in rows.iter().enumerate() {
            if row.depth == 0 {
                total = row.amount;
            } else if row.depth <= depth {
                add_account_row(&mut builder, &row.account, row.depth, row.amount);
                let expanded = rows
                    .get(index + 1)
                    .is_some_and(|child| child.depth > row.depth && child.depth <= depth);
                if expanded && !row.direct.is_zero() {
                    add_account_row(&mut builder, "(direct)", row.depth + 1, row.direct);
                }
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
    format!(
        "Profit and Loss | {}\n\n{table}\n",
        format_period(report.range, year)
    )
}

fn add_account_row(builder: &mut Builder, name: &str, depth: usize, amount: Decimal) {
    builder.push_record([
        format!("{}{name}", "  ".repeat(depth)),
        format_report_amount(amount),
    ]);
}

fn format_period(range: DateRange, year: i32) -> String {
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

pub fn format_bs(report: &BsReport, year: i32, depth: usize) -> String {
    let mut builder = Builder::default();
    builder.push_record(["Account", "Amount"]);
    let mut rows = report.rows.iter().peekable();
    let mut liabilities_and_equity = Decimal::ZERO;
    while let Some(root) = rows.next() {
        if root.account != "Assets" {
            builder.push_record(["", ""]);
            liabilities_and_equity += root.amount;
        }
        if depth > 0 {
            builder.push_record([root.account.to_uppercase(), String::new()]);
        }
        while rows.peek().is_some_and(|row| row.depth > 0) {
            let row = rows.next().expect("peek found a detail row");
            if row.depth <= depth {
                add_account_row(&mut builder, &row.account, row.depth, row.amount);
                let expanded = rows
                    .peek()
                    .is_some_and(|child| child.depth > row.depth && child.depth <= depth);
                if expanded && !row.direct.is_zero() {
                    add_account_row(&mut builder, "(direct)", row.depth + 1, row.direct);
                }
            }
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
    let at = report
        .range
        .to
        .map(format_date)
        .unwrap_or_else(|| format!("31 Dec {year:04}"));
    format!("Balance Sheet | As of {at}\n\n{table}\n")
}
