use super::{Report, RowKind};

pub(super) fn report(report: &Report) -> Result<String, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    let mut header = vec!["account", "depth", "type"];
    header.extend(report.columns.iter().map(String::as_str));
    writer.write_record(header)?;
    for row in &report.rows {
        let kind = match row.kind {
            RowKind::Account => "account",
            RowKind::Subtotal => "subtotal",
            RowKind::Direct => "direct",
            RowKind::Result => "result",
        };
        let mut record = vec![row.path.clone(), row.depth.to_string(), kind.to_string()];
        record.extend(row.amounts.iter().map(|amount| format!("{amount:.2}")));
        writer.write_record(record)?;
    }
    let bytes = writer.into_inner().map_err(|error| error.into_error())?;
    Ok(String::from_utf8(bytes).expect("CSV fields are UTF-8"))
}
