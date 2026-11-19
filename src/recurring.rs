use crate::models::{Journal, RecurrenceFrequency, RecurringTransaction, Transaction};
use crate::validation::validate_transaction;
use jiff::{Span, civil::Date};

fn next_occurrence(template: &RecurringTransaction, current: Date) -> Result<Date, String> {
    let next = match template.frequency {
        RecurrenceFrequency::Daily => current.checked_add(Span::new().days(1)),
        RecurrenceFrequency::Weekly => current.checked_add(Span::new().days(7)),
        RecurrenceFrequency::Biweekly => current.checked_add(Span::new().days(14)),
        RecurrenceFrequency::Monthly => {
            let first =
                Date::new(current.year(), current.month(), 1).map_err(|error| error.to_string())?;
            let next_month = first
                .checked_add(Span::new().months(1))
                .map_err(|error| format!("cannot advance recurring date: {error}"))?;
            // Always use the original day, so Jan 31 -> Feb 28 -> Mar 31.
            let day = template.start_date.day().min(next_month.days_in_month());
            Date::new(next_month.year(), next_month.month(), day)
        }
    };
    next.map_err(|error| format!("cannot advance recurring date: {error}"))
}

pub(crate) fn post_due_transactions(journal: &mut Journal, through: Date) -> Result<(), String> {
    if i32::from(through.year()) != journal.metadata.year {
        return Err(format!(
            "posting cutoff {through} is outside journal year {}",
            journal.metadata.year
        ));
    }
    let mut transactions = Vec::new();
    let mut next_dates = Vec::new();
    for template in &journal.recurring_transactions {
        let mut date = template.next_date;
        while date <= through {
            let transaction = Transaction::new(
                date,
                template.description.clone(),
                template.postings.clone(),
            );
            validate_transaction(journal, &transaction)
                .map_err(|error| format!("invalid recurring transaction: {error}"))?;
            transactions.push(transaction);
            date = next_occurrence(template, date)?;
        }
        next_dates.push(date);
    }
    // Generate and validate everything before changing transactions or cursors.
    // Stable sorting keeps template order for occurrences on the same date.
    transactions.sort_by_key(|transaction| transaction.date);
    journal.transactions.extend(transactions);
    for (template, next_date) in journal.recurring_transactions.iter_mut().zip(next_dates) {
        template.next_date = next_date;
    }
    Ok(())
}
