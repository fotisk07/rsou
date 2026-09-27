use crate::validation::{ValidationError, validate_transaction};
use jiff::civil::Date;
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

    pub fn find_account(&self, name: &str) -> Option<&Account> {
        let mut parts = name.split("::");

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
    pub date: Date,
    pub description: String,
    pub postings: Vec<Posting>,
}

#[derive(Debug)]
pub struct Journal {
    pub accounts: Accounts,
    pub transactions: Vec<Transaction>,
}
impl Journal {
    pub fn post(&mut self, transaction: Transaction) -> Result<(), ValidationError> {
        validate_transaction(&self.accounts, &transaction)?;
        self.transactions.push(transaction);
        Ok(())
    }
    pub fn transactions_for(&self, path: &str) -> Option<Vec<&Transaction>> {
        self.accounts.find_account(path)?; //discard account if not in the journal

        let child_prefix = format!("{path}::");
        Some(
            self.transactions
                .iter()
                .filter(|transaction| {
                    transaction.postings.iter().any(|posting| {
                        posting.account == path || posting.account.starts_with(&child_prefix)
                    })
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::models::*;
    use crate::parser::parse_journal;
    use jiff::civil::date;

    #[test]
    fn test_find_account() {
        let mut accounts = Accounts::new();
        accounts.assets.children.push(Account {
            name: "test".to_string(),
            children: vec![],
        });

        let account = accounts.find_account("Assets::test").unwrap();
        assert_eq!(account.name, "test");
    }

    #[test]
    fn test_valid_transaction() {
        let mut accounts = Accounts::new();

        accounts.assets.children.push(Account {
            name: "acc1".to_string(),
            children: vec![],
        });

        accounts.expenses.children.push(Account {
            name: "acc2".to_string(),
            children: vec![],
        });

        let valid_transaction = Transaction {
            date: date(2026, 9, 25),
            description: "Lunch".to_string(),
            postings: vec![
                Posting {
                    account: "Assets::acc1".to_string(),
                    amount: Decimal::from(-10),
                },
                Posting {
                    account: "Expenses::acc2".to_string(),
                    amount: Decimal::from(10),
                },
            ],
        };
        let mut journal = Journal {
            accounts,
            transactions: vec![],
        };
        assert!(journal.post(valid_transaction).is_ok());

        assert_eq!(journal.transactions.len(), 1);
    }
    #[test]
    fn test_invalid_transaction() {
        let mut accounts = Accounts::new();

        accounts.assets.children.push(Account {
            name: "acc1".to_string(),
            children: vec![],
        });

        accounts.expenses.children.push(Account {
            name: "acc2".to_string(),
            children: vec![],
        });
        let mut journal = Journal {
            accounts,
            transactions: vec![],
        };
        let invalid_transaction = Transaction {
            date: date(2026, 9, 25),
            description: "Lunch".to_string(),
            postings: vec![
                Posting {
                    account: "Assets::acc1".to_string(),
                    amount: Decimal::from(-10),
                },
                Posting {
                    account: "Expenses::acc2".to_string(),
                    amount: Decimal::from(5),
                },
            ],
        };

        assert!(journal.post(invalid_transaction).is_err());
        assert_eq!(journal.transactions.len(), 0);
    }

    #[test]
    fn test_ledger() {
        let journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();

        // A parent includes transactions posted to its child account.
        let cash = journal.transactions_for("Assets::Cash").unwrap();
        assert_eq!(cash.len(), 3);

        let groceries = journal
            .transactions_for("Expenses::Food::Groceries")
            .unwrap();
        assert_eq!(groceries.len(), 2);
        assert_eq!(groceries[0].description, "Groceries and snacks");
        assert_eq!(groceries[1].description, "Small refund");

        // An unknown account is different from an existing account with no transactions.
        assert!(journal.transactions_for("Expenses::Missing").is_none());
        assert!(journal.transactions_for("Income").unwrap().is_empty());
    }

    #[test]
    fn ledger_does_not_match_similar_account_names() {
        let mut journal = parse_journal(include_str!("../test_data/split.sou")).unwrap();
        journal.accounts.expenses.children.push(Account {
            name: "Foodie".to_string(),
            children: vec![],
        });
        journal
            .post(Transaction {
                date: date(2026, 2, 5),
                description: "Foodie expense".to_string(),
                postings: vec![
                    Posting {
                        account: "Assets::Cash::Checking".to_string(),
                        amount: Decimal::from(-2),
                    },
                    Posting {
                        account: "Expenses::Foodie".to_string(),
                        amount: Decimal::from(2),
                    },
                ],
            })
            .unwrap();

        let food = journal.transactions_for("Expenses::Food").unwrap();
        assert_eq!(food.len(), 2);
        assert!(
            food.iter()
                .all(|transaction| transaction.description != "Foodie expense")
        );
    }
}
