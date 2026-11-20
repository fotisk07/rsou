use jiff::civil::Date;
use rust_decimal::Decimal;

#[derive(Debug, PartialEq)]
pub struct Accounts {
    assets: Account,
    liabilities: Account,
    equity: Account,
    income: Account,
    expenses: Account,
}
impl Accounts {
    pub fn new() -> Self {
        Self {
            assets: Account {
                name: "Assets".to_string(),
                children: vec![],
            },
            liabilities: Account {
                name: "Liabilities".to_string(),
                children: vec![],
            },
            equity: Account {
                name: "Equity".to_string(),
                children: vec![],
            },
            income: Account {
                name: "Income".to_string(),
                children: vec![],
            },
            expenses: Account {
                name: "Expenses".to_string(),
                children: vec![],
            },
        }
    }

    pub fn roots(&self) -> impl Iterator<Item = &Account> {
        [
            &self.assets,
            &self.liabilities,
            &self.equity,
            &self.income,
            &self.expenses,
        ]
        .into_iter()
    }

    pub fn root_mut(&mut self, name: &str) -> Option<&mut Account> {
        match name {
            "Assets" => Some(&mut self.assets),
            "Liabilities" => Some(&mut self.liabilities),
            "Equity" => Some(&mut self.equity),
            "Income" => Some(&mut self.income),
            "Expenses" => Some(&mut self.expenses),
            _ => None,
        }
    }

    pub fn find_account_mut(&mut self, path: &str) -> Option<&mut Account> {
        let mut parts = path.split('/');
        let mut current = self.root_mut(parts.next()?)?;
        for part in parts {
            current = current
                .children
                .iter_mut()
                .find(|child| child.name == part)?;
        }
        Some(current)
    }

    pub fn find_account(&self, name: &str) -> Option<&Account> {
        let mut parts = name.split('/');

        let account_type = parts.next()?;

        let mut current = match account_type {
            "Assets" => &self.assets,
            "Equity" => &self.equity,
            "Liabilities" => &self.liabilities,
            "Income" => &self.income,
            "Expenses" => &self.expenses,
            _ => return None,
        };

        for part in parts {
            current = current.children.iter().find(|child| child.name == part)?;
        }

        Some(current)
    }
}
#[derive(Debug, PartialEq)]
pub struct Account {
    pub name: String,
    pub children: Vec<Account>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Posting {
    pub account: String,
    pub amount: Decimal,
}

#[derive(Debug, PartialEq)]
pub struct Transaction {
    pub date: Date,
    pub description: String,
    pub postings: Vec<Posting>,
}
impl Transaction {
    pub fn new(date: Date, description: String, postings: Vec<Posting>) -> Self {
        Self {
            date,
            description,
            postings,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct JournalMetadata {
    pub name: String,
    pub year: i32,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
}

impl std::fmt::Display for RecurrenceFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Biweekly => "biweekly",
            Self::Monthly => "monthly",
        })
    }
}

impl std::str::FromStr for RecurrenceFrequency {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "biweekly" => Ok(Self::Biweekly),
            "monthly" => Ok(Self::Monthly),
            _ => Err(format!(
                "unknown recurrence frequency `{value}`; expected daily, weekly, biweekly, or monthly"
            )),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RecurringTransaction {
    pub frequency: RecurrenceFrequency,
    pub start_date: Date,
    pub next_date: Date,
    pub description: String,
    pub postings: Vec<Posting>,
}

#[derive(Debug, PartialEq)]
pub struct AccountAlias {
    pub name: String,
    pub account: String,
}

#[derive(Debug, PartialEq)]
pub struct Journal {
    pub metadata: JournalMetadata,
    pub accounts: Accounts,
    pub aliases: Vec<AccountAlias>,
    pub transactions: Vec<Transaction>,
    pub recurring_transactions: Vec<RecurringTransaction>,
}
impl Journal {
    pub fn resolve_account<'a>(&'a self, reference: &'a str) -> &'a str {
        self.aliases
            .iter()
            .find(|alias| alias.name == reference)
            .map_or(reference, |alias| alias.account.as_str())
    }

    pub fn sort_transactions(&mut self) {
        self.transactions
            .sort_by_key(|transaction| transaction.date);
    }
}
