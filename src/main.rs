use regex::Regex;
use std::fs;

#[derive(Debug)]
struct Accounts {
    assets: Account,
    liabilities: Account,
    equity: Account,
    income: Account,
    expenses: Account,
}
impl Accounts {
    fn new() -> Self {
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

    fn root_mut(&mut self, name: &str) -> Option<&mut Account> {
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
struct Account {
    name: String,
    children: Vec<Account>,
}

fn parse_accounts(account_text: &String) -> Accounts {
    let mut accounts = Accounts::new();
    let mut stack: Vec<String> = Vec::new();

    for line in account_text.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let depth = line.chars().take_while(|c| *c == ' ').count() / 2;
        let name = line.trim().to_string();

        stack.truncate(depth);
        stack.push(name.clone());

        if depth == 0 {
            accounts.root_mut(&name).expect("Invalid root account");
            continue;
        }

        let root_name = &stack[0];
        let mut parent = accounts.root_mut(root_name).expect("Unknown root account");

        for parent_name in &stack[1..depth] {
            parent = parent
                .children
                .iter_mut()
                .find(|account| account.name == *parent_name)
                .expect("Parent account not found");
        }

        parent.children.push(Account {
            name,
            children: vec![],
        });
    }
    accounts
}

fn main() {
    let file_path = "journal.sou";
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");
    let re = Regex::new(r"(?s)\[ACCOUNTS\]\s*(.*?)\s*\[TRANSACTIONS\]").unwrap();
    let accounts_str = re.captures(&contents).unwrap().get(1).unwrap().as_str();

    let accounts = parse_accounts(&accounts_str.to_string());

    dbg!(accounts);
}
