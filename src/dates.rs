use crate::models::{Journal, Transaction};
use jiff::{Span, civil::Date};

#[derive(Clone, Copy, Default)]
pub struct DateRange {
    pub from: Option<Date>,
    pub to: Option<Date>,
}

#[derive(Clone, Copy)]
pub enum PeriodBreakdown {
    Month,
    Quarter,
}

impl PeriodBreakdown {
    /// Split the selected range at calendar boundaries, clipping partial periods.
    pub fn periods(self, range: DateRange, year: i32) -> Result<Vec<(String, DateRange)>, String> {
        let year = i16::try_from(year).map_err(|_| format!("invalid journal year {year}"))?;
        let from = range
            .from
            .unwrap_or(Date::new(year, 1, 1).map_err(|e| e.to_string())?);
        let to = range
            .to
            .unwrap_or(Date::new(year, 12, 31).map_err(|e| e.to_string())?);
        let width = match self {
            Self::Month => 1,
            Self::Quarter => 3,
        };
        let first_month = ((from.month() - 1) / width) * width + 1;
        let mut periods = Vec::new();
        for month in (first_month..=to.month()).step_by(width as usize) {
            let start = Date::new(year, month, 1).map_err(|e| e.to_string())?;
            let end = Date::new(year, month + width - 1, 1)
                .map_err(|e| e.to_string())?
                .last_of_month();
            let clipped = DateRange {
                from: Some(start.max(from)),
                to: Some(end.min(to)),
            };
            let mut label = match self {
                Self::Month => start.strftime("%b").to_string(),
                Self::Quarter => format!("Q{}", (month - 1) / 3 + 1),
            };
            if from > start || to < end {
                label.push_str(" (partial)");
            }
            periods.push((label, clipped));
        }
        Ok(periods)
    }
}

/// An unresolved date selection, independent of CLI parsing.
#[derive(Default)]
pub struct DateRequest<'a> {
    pub from: Option<&'a str>,
    pub to: Option<&'a str>,
    pub shortcut: Option<PeriodShortcut>,
    /// Default to the current month only for current-year journals.
    /// Cumulative reports use the resolved upper bound and ignore the lower bound.
    pub default_to_current_month: bool,
}

impl DateRequest<'_> {
    /// Resolve explicit DD-MM bounds or a shortcut using the same calendar rules.
    pub fn resolve(self, year: i32, today: Date) -> Result<DateRange, String> {
        if let Some(shortcut) = self.shortcut {
            if self.from.is_some() || self.to.is_some() {
                return Err("date shortcuts cannot be combined with from/to dates".to_string());
            }
            return shortcut.resolve(year, today);
        }
        if self.default_to_current_month
            && self.from.is_none()
            && self.to.is_none()
            && year == i32::from(today.year())
        {
            return PeriodShortcut::Month(today.month() as u8).resolve(year, today);
        }
        let from = self.from.map(|value| parse_date(year, value)).transpose()?;
        let to = self.to.map(|value| parse_date(year, value)).transpose()?;
        if let (Some(from), Some(to)) = (from, to)
            && from > to
        {
            return Err("from date cannot be after to date".to_string());
        }
        Ok(DateRange { from, to })
    }
}

impl DateRange {
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
    Year,
    Month(u8),
    Quarter(u8),
    CurrentMonth,
    CurrentQuarter,
    MonthToDate,
    QuarterToDate,
    YearToDate,
    LastMonth,
    Today,
    WeekEnd,
    MonthEnd,
}

impl PeriodShortcut {
    /// Resolve calendar shortcuts independently of CLI parsing and the system clock.
    fn resolve(self, year: i32, today: Date) -> Result<DateRange, String> {
        if !matches!(self, Self::Year | Self::Month(_) | Self::Quarter(_)) && year != i32::from(today.year()) {
            return Err(format!(
                "relative date shortcuts require journal year {} (found {year})",
                today.year()
            ));
        }

        let current_month = today.month() as u8;
        let quarter_start = |month: u8| ((month - 1) / 3) * 3 + 1;
        let (first_month, last_month) = match self {
            Self::Year => (1, 12),
            Self::Month(month) => (month, month),
            Self::Quarter(quarter) => {
                if !(1..=4).contains(&quarter) {
                    return Err("quarter must be between 1 and 4".to_string());
                }
                let first = (quarter - 1) * 3 + 1;
                (first, first + 2)
            }
            Self::CurrentMonth | Self::MonthToDate => (current_month, current_month),
            Self::CurrentQuarter => {
                let first = quarter_start(current_month);
                (first, first + 2)
            },
            Self::QuarterToDate => (quarter_start(current_month), current_month),
            Self::YearToDate => (1, current_month),
            Self::WeekEnd => {
                let year_end =
                    Date::new(today.year(), 12, 31).map_err(|error| error.to_string())?;
                let days = i64::from(6 - today.weekday().to_monday_zero_offset());
                let to = today
                    .checked_add(Span::new().days(days))
                    .map(|date| date.min(year_end))
                    .unwrap_or(year_end);
                return Ok(DateRange {
                    from: None,
                    to: Some(to),
                });
            }
            Self::MonthEnd => {
                return Ok(DateRange {
                    from: None,
                    to: Some(today.last_of_month()),
                });
            }
            Self::Today => {
                return Ok(DateRange {
                    from: None,
                    to: Some(today),
                });
            }
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

pub(crate) fn parse_date(year: i32, value: &str) -> Result<Date, String> {
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
