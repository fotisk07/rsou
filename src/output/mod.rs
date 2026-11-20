mod csv;
mod text;
mod xlsx;

pub use text::{format_accounts, format_ledger, format_recurring};

use crate::{
    models::JournalMetadata,
    queries::{BsReport, PnlReport},
};
use rust_decimal::Decimal;

#[derive(Clone, Copy, clap::ValueEnum, Default)]
pub(crate) enum ReportFormat {
    #[default]
    Text,
    Xlsx,
    Csv,
}

#[derive(Clone, Copy, PartialEq)]
enum RowKind {
    Account,
    Subtotal,
    Direct,
    Result,
}

struct ReportRow {
    name: String,
    path: String,
    depth: usize,
    kind: RowKind,
    amounts: Vec<Decimal>,
}

struct Report {
    title: &'static str,
    period: String,
    columns: Vec<String>,
    rows: Vec<ReportRow>,
}

// Query adapters provide numbers; hierarchy and depth selection happen only here.
struct SourceRow {
    name: String,
    depth: usize,
    amounts: Vec<Decimal>,
    direct: Vec<Decimal>,
}

fn select_rows(source: &[SourceRow], depth: usize) -> Vec<ReportRow> {
    let mut visible: Vec<bool> = source
        .iter()
        .map(|row| {
            row.depth == 0
                || row
                    .amounts
                    .iter()
                    .chain(&row.direct)
                    .any(|amount| !amount.is_zero())
        })
        .collect();
    let mut parents = Vec::new();
    for (index, row) in source.iter().enumerate() {
        parents.truncate(row.depth);
        if visible[index] {
            for &parent in &parents {
                visible[parent] = true;
            }
        }
        parents.push(index);
    }
    let mut result = Vec::new();
    let mut path = Vec::new();
    for (index, row) in source.iter().enumerate() {
        path.truncate(row.depth);
        path.push(row.name.as_str());
        if !visible[index] || row.depth > depth {
            continue;
        }
        let expanded = source[index + 1..]
            .iter()
            .enumerate()
            .take_while(|(_, child)| child.depth > row.depth)
            .any(|(offset, child)| visible[index + 1 + offset] && child.depth <= depth);
        result.push(ReportRow {
            name: row.name.clone(),
            path: path.join("/"),
            depth: row.depth,
            kind: if row.depth == 0 || expanded {
                RowKind::Subtotal
            } else {
                RowKind::Account
            },
            amounts: row.amounts.clone(),
        });
        // Section-root direct values are already represented by section totals
        // (and the synthetic current-year result in balance-sheet equity).
        if row.depth > 0 && expanded && row.direct.iter().any(|amount| !amount.is_zero()) {
            result.push(ReportRow {
                name: "(direct)".to_string(),
                path: path.join("/"),
                depth: row.depth + 1,
                kind: RowKind::Direct,
                amounts: row.direct.clone(),
            });
        }
    }
    result
}

pub fn pnl(
    columns: &[(String, PnlReport)],
    metadata: &JournalMetadata,
    depth: usize,
    format: ReportFormat,
) -> Result<Option<String>, String> {
    // Breakdown queries append the full-period Total column last.
    let range = columns.last().expect("at least one P&L column").1.range;
    let source: Vec<_> = columns[0]
        .1
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| SourceRow {
            name: row.account.clone(),
            depth: row.depth,
            amounts: columns
                .iter()
                .map(|(_, report)| report.rows[index].amount)
                .collect(),
            direct: columns
                .iter()
                .map(|(_, report)| report.rows[index].direct)
                .collect(),
        })
        .collect();
    let mut rows = select_rows(&source, depth);
    rows.push(ReportRow {
        name: "Net Result".to_string(),
        path: "Net Result".to_string(),
        depth: 0,
        kind: RowKind::Result,
        amounts: columns
            .iter()
            .map(|(_, report)| report.net_result)
            .collect(),
    });
    render(
        &Report {
            title: "Profit and Loss",
            period: text::format_period(range, metadata.year),
            columns: columns.iter().map(|(label, _)| label.clone()).collect(),
            rows,
        },
        metadata,
        depth,
        format,
    )
}

pub fn bs(
    report: &BsReport,
    metadata: &JournalMetadata,
    depth: usize,
    format: ReportFormat,
) -> Result<Option<String>, String> {
    let source: Vec<_> = report
        .rows
        .iter()
        .map(|row| SourceRow {
            name: row.account.clone(),
            depth: row.depth,
            amounts: vec![row.amount],
            direct: vec![row.direct],
        })
        .collect();
    let total = report
        .rows
        .iter()
        .filter(|row| row.depth == 0 && row.account != "Assets")
        .map(|row| row.amount)
        .sum();
    let mut rows = select_rows(&source, depth);
    rows.push(ReportRow {
        name: "Total Liabilities and Equity".to_string(),
        path: "Total Liabilities and Equity".to_string(),
        depth: 0,
        kind: RowKind::Result,
        amounts: vec![total],
    });
    let at = report
        .range
        .to
        .map(|date| date.strftime("%d %b %Y").to_string())
        .unwrap_or_else(|| format!("31 Dec {:04}", metadata.year));
    render(
        &Report {
            title: "Balance Sheet",
            period: format!("As of {at}"),
            columns: vec!["Amount".to_string()],
            rows,
        },
        metadata,
        depth,
        format,
    )
}

fn render(
    report: &Report,
    metadata: &JournalMetadata,
    depth: usize,
    format: ReportFormat,
) -> Result<Option<String>, String> {
    use std::io::{IsTerminal, Write};
    match format {
        ReportFormat::Text => Ok(Some(text::report(report, depth))),
        ReportFormat::Csv => csv::report(report)
            .map(Some)
            .map_err(|error| format!("failed exporting report: {error}")),
        ReportFormat::Xlsx => {
            if std::io::stdout().is_terminal() {
                return Err(
                    "redirect XLSX output to a file, e.g. sou pnl --format xlsx > pnl.xlsx"
                        .to_string(),
                );
            }
            let bytes = xlsx::report(report, metadata)
                .map_err(|error| format!("failed exporting report: {error}"))?;
            std::io::stdout()
                .lock()
                .write_all(&bytes)
                .map_err(|error| format!("failed writing XLSX: {error}"))?;
            Ok(None)
        }
    }
}
