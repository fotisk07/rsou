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

#[derive(Clone, Copy)]
pub enum PeriodShortcut {
    Month(u8),
    Quarter(u8),
    MonthToDate,
    QuarterToDate,
    YearToDate,
    LastMonth,
}

impl PeriodShortcut {
    /// Resolve calendar shortcuts independently of CLI parsing and the system clock.
    pub fn resolve(self, year: i32, today: Date) -> Result<DateRange, String> {
        if !matches!(self, Self::Month(_) | Self::Quarter(_)) && year != i32::from(today.year()) {
            return Err(format!(
                "relative date shortcuts require journal year {} (found {year})",
                today.year()
            ));
        }

        let current_month = today.month() as u8;
        let quarter_start = |month: u8| ((month - 1) / 3) * 3 + 1;
        let (first_month, last_month) = match self {
            Self::Month(month) => (month, month),
            Self::Quarter(quarter) => {
                if !(1..=4).contains(&quarter) {
                    return Err("quarter must be between 1 and 4".to_string());
                }
                let first = (quarter - 1) * 3 + 1;
                (first, first + 2)
            }
            Self::MonthToDate => (current_month, current_month),
            Self::QuarterToDate => (quarter_start(current_month), current_month),
            Self::YearToDate => (1, current_month),
            Self::LastMonth => {
                if current_month == 1 {
                    return Err("last month belongs to the previous year; use a journal for that year with -m 12".to_string());
                }
                (current_month - 1, current_month - 1)
            }
        };
        if !(1..=12).contains(&first_month) || !(1..=12).contains(&last_month) {
            return Err("month must be between 1 and 12".to_string());
        }
        let year = i16::try_from(year).map_err(|_| format!("invalid journal year {year}"))?;
        let from = Date::new(year, first_month as i8, 1)
            .map_err(|_| format!("invalid journal year {year}"))?;
        let last = Date::new(year, last_month as i8, 1)
            .map_err(|_| format!("invalid journal year {year}"))?;
        let to = if matches!(
            self,
            Self::MonthToDate | Self::QuarterToDate | Self::YearToDate
        ) {
            today
        } else {
            last.last_of_month()
        };
        Ok(DateRange {
            from: Some(from),
            to: Some(to),
        })
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
