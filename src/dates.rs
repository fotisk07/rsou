use crate::models::{Journal, Transaction};
use jiff::civil::Date;

#[derive(Clone, Copy, Default)]
pub struct DateRange {
    pub from: Option<Date>,
    pub to: Option<Date>,
}

impl DateRange {
    /// Resolve inclusive DD-MM bounds using the journal year. Missing bounds are open.
    pub fn resolve(year: i32, from: Option<&str>, to: Option<&str>) -> Result<Self, String> {
        let from = from.map(|value| parse_date(year, value)).transpose()?;
        let to = to.map(|value| parse_date(year, value)).transpose()?;
        if let (Some(from), Some(to)) = (from, to)
            && from > to
        {
            return Err("from date cannot be after to date".to_string());
        }
        Ok(Self { from, to })
    }

    pub fn contains(self, date: Date) -> bool {
        self.from.is_none_or(|from| date >= from) && self.to.is_none_or(|to| date <= to)
    }

    pub fn before_start(self, date: Date) -> bool {
        self.from.is_some_and(|from| date < from)
    }

    pub fn transactions(self, journal: &Journal) -> impl Iterator<Item = &Transaction> {
        journal
            .transactions
            .iter()
            .filter(move |transaction| self.contains(transaction.date))
    }
}

fn parse_date(year: i32, value: &str) -> Result<Date, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b'-'
        || !bytes[..2].iter().all(u8::is_ascii_digit)
        || !bytes[3..].iter().all(u8::is_ascii_digit)
    {
        return Err(format!("invalid date `{value}`: expected DD-MM"));
    }
    let day = ((bytes[0] - b'0') * 10 + bytes[1] - b'0') as i8;
    let month = ((bytes[3] - b'0') * 10 + bytes[4] - b'0') as i8;
    let calendar_year = i16::try_from(year).map_err(|_| format!("invalid journal year {year}"))?;
    Date::new(calendar_year, month, day)
        .map_err(|_| format!("invalid date `{value}` for journal year {year}"))
}
