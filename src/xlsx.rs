use crate::{dates::DateRange, models::JournalMetadata, output, queries::PnlReport};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook, Worksheet, XlsxError};

/// Export the same account totals and depth selection as the terminal P&L.
pub fn pnl(
    columns: &[(String, PnlReport)],
    metadata: &JournalMetadata,
    range: DateRange,
    depth: usize,
) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Profit and Loss")?;
    sheet.set_column_width(0, 40)?;
    sheet.set_column_range_width(1, columns.len() as u16, 18)?;
    sheet.set_freeze_panes(4, 1)?;

    let title = Format::new().set_bold().set_font_size(18);
    let header = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE6EDF2));
    let subtotal = Format::new().set_bold();
    let result = header.clone().set_border_top(FormatBorder::Thin);
    sheet.write_string_with_format(0, 0, "Profit and Loss", &title)?;
    sheet.write_string(
        1,
        0,
        format!(
            "{} | {} | {}",
            metadata.name,
            output::format_period(range, metadata.year),
            metadata.currency
        ),
    )?;
    sheet.write_string_with_format(3, 0, "Account", &header)?;
    for (col, (label, _)) in columns.iter().enumerate() {
        sheet.write_string_with_format(3, col as u16 + 1, label, &header)?;
    }

    let rows = &columns[0].1.rows;
    let mut visible: Vec<bool> = (0..rows.len())
        .map(|index| {
            rows[index].depth == 0
                || columns.iter().any(|(_, report)| {
                    !report.rows[index].amount.is_zero() || !report.rows[index].direct.is_zero()
                })
        })
        .collect();
    let mut parents = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        parents.truncate(row.depth);
        if visible[index] {
            for &parent in &parents {
                visible[parent] = true;
            }
        }
        parents.push(index);
    }

    let mut line = 4;
    for (index, row) in rows.iter().enumerate() {
        if !visible[index] || row.depth > depth {
            continue;
        }
        if row.depth == 0 && index > 0 {
            line += 1;
        }
        let expanded = rows[index + 1..]
            .iter()
            .enumerate()
            .take_while(|(_, child)| child.depth > row.depth)
            .any(|(offset, child)| visible[index + 1 + offset] && child.depth <= depth);
        let style = if row.depth == 0 || expanded {
            &subtotal
        } else {
            &Format::new()
        };
        let label_style = style.clone().set_indent(row.depth.min(250) as u8);
        sheet.write_string_with_format(line, 0, &row.account, &label_style)?;
        write_amounts(
            sheet,
            line,
            columns.iter().map(|(_, report)| report.rows[index].amount),
            style,
        )?;
        line += 1;
        if expanded
            && columns
                .iter()
                .any(|(_, report)| !report.rows[index].direct.is_zero())
        {
            let direct_style = Format::new().set_indent((row.depth + 1).min(250) as u8);
            sheet.write_string_with_format(line, 0, "(direct)", &direct_style)?;
            write_amounts(
                sheet,
                line,
                columns.iter().map(|(_, report)| report.rows[index].direct),
                &Format::new(),
            )?;
            line += 1;
        }
    }
    line += 1;
    sheet.write_string_with_format(line, 0, "Net Result", &result)?;
    write_amounts(
        sheet,
        line,
        columns.iter().map(|(_, report)| report.net_result),
        &result,
    )?;
    sheet.write_string(
        line + 2,
        0,
        "Bold parent rows are subtotals; do not add them to their children.",
    )?;
    workbook.save_to_buffer()
}

fn write_amounts(
    sheet: &mut Worksheet,
    line: u32,
    amounts: impl Iterator<Item = Decimal>,
    style: &Format,
) -> Result<(), XlsxError> {
    let style = style
        .clone()
        .set_num_format("#,##0.00;[Red](#,##0.00);\"–\"");
    for (col, amount) in amounts.enumerate() {
        let value = amount.to_f64().ok_or_else(|| {
            XlsxError::ParameterError(
                "amount cannot be represented as a spreadsheet number".to_string(),
            )
        })?;
        sheet.write_number_with_format(line, col as u16 + 1, value, &style)?;
    }
    Ok(())
}
