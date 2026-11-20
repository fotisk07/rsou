use super::{Report, ReportRow, RowKind, text::format_report_amount};
use crate::models::JournalMetadata;
use genpdf::{
    Alignment, Context, Element, Margins, PageDecorator, Position, RenderResult, Size,
    elements::{Break, LinearLayout, Paragraph},
    error::{Error, ErrorKind},
    fonts::{FontCache, FontData, FontFamily},
    render::Area,
    style::{Color, Style},
};

const FONT_SIZE: u8 = 10;
const MARGIN: f64 = 20.0;
const LINE_HEIGHT: f64 = 4.2;
const INK: Color = Color::Rgb(46, 43, 38);

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Heading,
    Detail,
    Subtotal,
    Result,
}

struct Row {
    label: String,
    depth: usize,
    amounts: Vec<String>,
    kind: Kind,
}

fn amount(value: rust_decimal::Decimal) -> String {
    if value.is_zero() {
        return "–".to_string();
    }
    let formatted = format_report_amount(value);
    formatted
        .strip_prefix('-')
        .map_or_else(|| formatted.clone(), |s| format!("({s})"))
}

fn rows(report: &Report) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut section: Option<&ReportRow> = None;
    for row in &report.rows {
        if row.depth == 0
            && let Some(root) = section.take()
        {
            rows.push(Row {
                label: format!("Total {}", root.name),
                depth: 0,
                amounts: root.amounts.iter().copied().map(amount).collect(),
                kind: Kind::Subtotal,
            });
        }
        let heading = row.depth == 0 && row.kind != RowKind::Result;
        rows.push(Row {
            label: if heading {
                row.name.to_uppercase()
            } else {
                row.name.clone()
            },
            depth: row.depth,
            amounts: if heading {
                Vec::new()
            } else {
                row.amounts.iter().copied().map(amount).collect()
            },
            kind: if heading {
                Kind::Heading
            } else if row.kind == RowKind::Result {
                Kind::Result
            } else if row.kind == RowKind::Subtotal {
                Kind::Subtotal
            } else {
                Kind::Detail
            },
        });
        if heading {
            section = Some(row);
        }
    }
    rows
}

fn width(cache: &FontCache, text: &str, style: Style) -> f64 {
    cache
        .default_font_family()
        .get(style)
        .str_width(cache, text, style.font_size())
        .into()
}

pub(super) fn report(report: &Report, metadata: &JournalMetadata) -> Result<Vec<u8>, String> {
    let regular = FontData::new(
        include_bytes!("../../assets/fonts/LiberationSerif-Regular.ttf").to_vec(),
        None,
    )
    .map_err(|error| error.to_string())?;
    let bold = FontData::new(
        include_bytes!("../../assets/fonts/LiberationSerif-Bold.ttf").to_vec(),
        None,
    )
    .map_err(|error| error.to_string())?;
    let mut document = genpdf::Document::new(FontFamily {
        regular: regular.clone(),
        italic: regular,
        bold: bold.clone(),
        bold_italic: bold,
    });
    document.set_title(format!("{} — {}", metadata.name, report.title));
    document.set_font_size(FONT_SIZE);
    document.set_minimal_conformance();
    let rows = rows(report);
    let cache = document.font_cache();
    // Do not silently drop characters that the embedded typeface cannot render.
    for text in [&metadata.name, &metadata.currency, &report.period]
        .into_iter()
        .chain(metadata.author.iter())
        .chain(report.columns.iter())
        .chain(rows.iter().map(|row| &row.label))
    {
        for font in [
            cache.default_font_family().regular,
            cache.default_font_family().bold,
        ] {
            if let Some(character) = text
                .chars()
                .find(|&c| !c.is_control() && font.glyph_ids(cache, [c])[0] == 0)
            {
                return Err(format!(
                    "PDF font cannot render {character:?}; use XLSX or CSV for this report"
                ));
            }
        }
    }
    let bold = Style::new().with_font_size(FONT_SIZE).bold();
    let numeric: Vec<f64> = report
        .columns
        .iter()
        .enumerate()
        .map(|(index, label)| {
            rows.iter()
                .filter_map(|row| row.amounts.get(index))
                .map(|value| width(cache, value, bold))
                .fold(width(cache, label, bold), f64::max)
                + 6.0
        })
        .collect();
    // Comparable periods get equal-width columns, even when some are all zero.
    let numeric = vec![numeric.iter().copied().fold(0.0, f64::max); numeric.len()];
    let numbers: f64 = numeric.iter().sum();
    // Choose the smallest standard sheet that leaves readable space for accounts.
    let (paper_width, paper_height) = [(210.0, 297.0), (297.0, 210.0), (420.0, 297.0)]
        .into_iter()
        .find(|(w, _)| w - 2.0 * MARGIN - numbers >= 65.0)
        .ok_or("report is too wide for PDF; select fewer periods or use XLSX")?;
    let mut widths = vec![paper_width - 2.0 * MARGIN - numbers];
    widths.extend(numeric);
    document.set_paper_size(Size::new(paper_width, paper_height));
    document.set_page_decorator(StatementPage {
        page: 0,
        title: report.title,
        name: metadata.name.clone(),
        author: metadata.author.clone(),
        period: report.period.clone(),
        currency: metadata.currency.clone(),
        columns: report.columns.clone(),
        widths: widths.clone(),
    });
    document.push(StatementTable {
        rows,
        widths,
        next: 0,
    });
    let mut bytes = Vec::new();
    document
        .render(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

struct StatementPage {
    page: usize,
    title: &'static str,
    name: String,
    author: Option<String>,
    period: String,
    currency: String,
    columns: Vec<String>,
    widths: Vec<f64>,
}

impl PageDecorator for StatementPage {
    fn decorate_page<'a>(
        &mut self,
        context: &Context,
        mut area: Area<'a>,
        style: Style,
    ) -> Result<Area<'a>, Error> {
        self.page += 1;
        area.add_margins(Margins::all(MARGIN));
        let height: f64 = area.size().height.into();
        let page_width: f64 = area.size().width.into();
        let footer = Style::new()
            .with_font_size(8)
            .with_color(Color::Greyscale(100));
        rule(&area, height - 8.0, page_width);
        area.print_str(
            &context.font_cache,
            Position::new(0, height - 5.0),
            footer,
            self.title,
        )?;
        let page = format!("Page {}", self.page);
        area.print_str(
            &context.font_cache,
            Position::new(
                page_width - width(&context.font_cache, &page, footer),
                height - 5.0,
            ),
            footer,
            page,
        )?;
        area.set_height((height - 14.0).into());
        let mut header = LinearLayout::vertical();
        header.push(
            Paragraph::new(self.name.to_uppercase())
                .aligned(Alignment::Center)
                .styled(Style::new().bold().with_font_size(10).with_color(INK)),
        );
        if let Some(author) = &self.author {
            header.push(
                Paragraph::new(author).aligned(Alignment::Center).styled(
                    Style::new()
                        .with_font_size(10)
                        .with_color(Color::Greyscale(95)),
                ),
            );
        }
        header.push(Break::new(0.5));
        header.push(
            Paragraph::new(self.title)
                .aligned(Alignment::Center)
                .styled(Style::new().with_font_size(24).with_color(INK)),
        );
        header.push(Break::new(0.2));
        header.push(
            Paragraph::new(&self.period)
                .aligned(Alignment::Center)
                .styled(Style::new().with_font_size(10).with_color(INK)),
        );
        header.push(Break::new(0.5));
        header.push(
            Paragraph::new(format!("Amounts in {}", self.currency))
                .aligned(Alignment::Right)
                .styled(footer),
        );
        header.push(Break::new(0.3));
        let rendered = header.render(context, area.clone(), style)?;
        if rendered.has_more {
            return Err(Error::new(
                "report heading is too tall for the page",
                ErrorKind::InvalidData,
            ));
        }
        area.add_offset(Position::new(0, rendered.size.height));
        rule(&area, 0.0, page_width);
        let header_style = Style::new()
            .with_font_size(FONT_SIZE)
            .bold()
            .with_color(INK);
        area.print_str(
            &context.font_cache,
            Position::new(0, 2),
            header_style,
            "Account",
        )?;
        let mut x = self.widths[0];
        for (index, label) in self.columns.iter().enumerate() {
            x += self.widths[index + 1];
            area.print_str(
                &context.font_cache,
                Position::new(x - width(&context.font_cache, label, header_style), 2),
                header_style,
                label,
            )?;
        }
        rule(&area, 9.0, page_width);
        area.add_offset(Position::new(0, 10));
        Ok(area)
    }
}

fn rule(area: &Area<'_>, y: f64, width: f64) {
    area.draw_line(
        vec![Position::new(0, y), Position::new(width, y)],
        Style::new().with_color(Color::Rgb(170, 164, 153)),
    );
}

struct StatementTable {
    rows: Vec<Row>,
    widths: Vec<f64>,
    next: usize,
}

impl StatementTable {
    fn layout(&self, index: usize, cache: &FontCache) -> (Vec<String>, Style, f64, f64) {
        let row = &self.rows[index];
        let mut style = Style::new().with_font_size(FONT_SIZE).with_color(INK);
        if row.kind != Kind::Detail {
            style = style.bold();
        }
        let indent = (row.depth as f64 * 4.0).min(self.widths[0] / 3.0);
        let available = self.widths[0] - indent - 5.0;
        let mut lines = Vec::new();
        let mut line = String::new();
        // Character wrapping also handles long unbroken account names.
        for character in row.label.chars() {
            let mut candidate = line.clone();
            candidate.push(character);
            if !line.is_empty() && width(cache, &candidate, style) > available {
                lines.push(line);
                line = String::new();
            }
            line.push(character);
        }
        lines.push(line);
        let padding = match row.kind {
            Kind::Heading => 4.5,
            Kind::Result => 4.0,
            _ => 1.8,
        };
        let height = lines.len() as f64 * LINE_HEIGHT + padding;
        (lines, style, indent, height)
    }
}

impl Element for StatementTable {
    fn render(
        &mut self,
        context: &Context,
        area: Area<'_>,
        _: Style,
    ) -> Result<RenderResult, Error> {
        let available: f64 = area.size().height.into();
        let page_width: f64 = area.size().width.into();
        let mut y = 0.0;
        while self.next < self.rows.len() {
            let row = &self.rows[self.next];
            let (lines, style, indent, height) = self.layout(self.next, &context.font_cache);
            // Never leave a section heading stranded at the bottom of a page.
            let keep_next = self.rows.get(self.next + 1).is_some_and(|next| {
                row.kind == Kind::Heading
                    || (row.kind == Kind::Subtotal && next.kind == Kind::Result)
            });
            let following = if keep_next {
                self.layout(self.next + 1, &context.font_cache).3
            } else {
                0.0
            };
            if y + height + following > available {
                if y == 0.0 {
                    return Err(Error::new(
                        "account row is too tall for the PDF page",
                        ErrorKind::InvalidData,
                    ));
                }
                break;
            }
            if matches!(row.kind, Kind::Subtotal | Kind::Result) {
                rule(&area, y, page_width);
            }
            let top = y + if row.kind == Kind::Heading { 2.0 } else { 0.8 };
            for (line, label) in lines.iter().enumerate() {
                area.print_str(
                    &context.font_cache,
                    Position::new(indent, top + line as f64 * LINE_HEIGHT),
                    style,
                    label,
                )?;
            }
            let mut x = self.widths[0];
            for (index, value) in row.amounts.iter().enumerate() {
                x += self.widths[index + 1];
                area.print_str(
                    &context.font_cache,
                    Position::new(x - width(&context.font_cache, value, style), top),
                    style,
                    value,
                )?;
            }
            y += height;
            if row.kind == Kind::Result {
                rule(&area, y - 0.8, page_width);
                rule(&area, y, page_width);
            }
            self.next += 1;
        }
        Ok(RenderResult {
            size: Size::new(page_width, y),
            has_more: self.next < self.rows.len(),
        })
    }
}
