use super::{Report, RowKind};
use crate::models::JournalMetadata;
use rust_decimal::prelude::ToPrimitive;
use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook, XlsxError};

pub(super) fn report(report: &Report, metadata: &JournalMetadata) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name(report.title)?;
    sheet.set_column_width(0, 40)?;
    sheet.set_column_range_width(1, report.columns.len() as u16, 18)?;
    sheet.set_freeze_panes(4, 1)?;
    let title = Format::new().set_bold().set_font_size(18);
    let header = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE6EDF2));
    let subtotal = Format::new().set_bold();
    let result = header.clone().set_border_top(FormatBorder::Thin);
    let plain = Format::new();
    sheet.write_string_with_format(0, 0, report.title, &title)?;
    sheet.write_string(
        1,
        0,
        format!(
            "{} | {} | {}",
            metadata.name, report.period, metadata.currency
        ),
    )?;
    sheet.write_string_with_format(3, 0, "Account", &header)?;
    for (col, label) in report.columns.iter().enumerate() {
        sheet.write_string_with_format(3, col as u16 + 1, label, &header)?;
    }
    let mut line = 4;
    for (index, row) in report.rows.iter().enumerate() {
        if row.depth == 0 && index > 0 {
            line += 1;
        }
        let style = match row.kind {
            RowKind::Subtotal => &subtotal,
            RowKind::Result => &result,
            _ => &plain,
        };
        let label_style = style.clone().set_indent(row.depth.min(250) as u8);
        sheet.write_string_with_format(line, 0, &row.name, &label_style)?;
        let number_style = style
            .clone()
            .set_num_format("#,##0.00;[Red](#,##0.00);\"–\"");
        for (col, amount) in row.amounts.iter().enumerate() {
            let value = amount.to_f64().ok_or_else(|| {
                XlsxError::ParameterError(
                    "amount cannot be represented as a spreadsheet number".to_string(),
                )
            })?;
            sheet.write_number_with_format(line, col as u16 + 1, value, &number_style)?;
        }
        line += 1;
    }
    sheet.write_string(
        line + 1,
        0,
        "Bold parent rows are subtotals; do not add them to their children.",
    )?;
    workbook.save_to_buffer()
}
