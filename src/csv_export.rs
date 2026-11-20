use crate::queries::PnlReport;

/// Flat report data: full account paths preserve hierarchy, and row types
/// distinguish aggregate totals from amounts that can be added together.
pub fn pnl(columns: &[(String, PnlReport)], depth: usize) -> Result<String, csv::Error> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    let mut header = vec!["account", "depth", "type"];
    header.extend(columns.iter().map(|(label, _)| label.as_str()));
    writer.write_record(header)?;

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

    let mut path = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        path.truncate(row.depth);
        path.push(row.account.as_str());
        if !visible[index] || row.depth > depth {
            continue;
        }
        let expanded = rows[index + 1..]
            .iter()
            .enumerate()
            .take_while(|(_, child)| child.depth > row.depth)
            .any(|(offset, child)| visible[index + 1 + offset] && child.depth <= depth);
        let mut record = vec![
            path.join("/"),
            row.depth.to_string(),
            if row.depth == 0 || expanded {
                "subtotal"
            } else {
                "account"
            }
            .to_string(),
        ];
        record.extend(
            columns
                .iter()
                .map(|(_, report)| format!("{:.2}", report.rows[index].amount)),
        );
        writer.write_record(record)?;
        if expanded
            && columns
                .iter()
                .any(|(_, report)| !report.rows[index].direct.is_zero())
        {
            let mut record = vec![
                path.join("/"),
                (row.depth + 1).to_string(),
                "direct".to_string(),
            ];
            record.extend(
                columns
                    .iter()
                    .map(|(_, report)| format!("{:.2}", report.rows[index].direct)),
            );
            writer.write_record(record)?;
        }
    }
    let mut record = vec![
        "Net Result".to_string(),
        "0".to_string(),
        "result".to_string(),
    ];
    record.extend(
        columns
            .iter()
            .map(|(_, report)| format!("{:.2}", report.net_result)),
    );
    writer.write_record(record)?;
    let bytes = writer.into_inner().map_err(|error| error.into_error())?;
    // All fields are Rust strings, so the CSV writer always produces UTF-8.
    Ok(String::from_utf8(bytes).expect("CSV fields are UTF-8"))
}
