use chrono::NaiveDate;
use rust_decimal::Decimal;

#[derive(Debug)]
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
}
#[derive(Debug)]
pub struct Account {
    pub name: String,
    pub children: Vec<Account>,
}

#[derive(Debug)]
pub struct Posting {
    pub account: String,
    pub amount: Decimal,
}

#[derive(Debug)]
pub struct Transaction {
    pub date: NaiveDate,
    pub description: String,
    pub postings: Vec<Posting>,
}

#[derive(Debug)]
pub struct Journal {
    pub accounts: Accounts,
    pub transactions: Vec<Transaction>,
}
