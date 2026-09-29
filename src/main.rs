mod cli;
mod models;
mod parser;
mod queries;
mod validation;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rsou")]
#[command(about = "Personal accounting CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Check {
        #[arg(short, long, default_value = "journal.sou")]
        journal_path: String,
    },
    Ledger {
        #[arg(short, long, default_value = "journal.sou")]
        journal_path: String,
    },
}
fn main() {
    let cli = Cli::parse();

    let journal_path = match &cli.command {
        Commands::Check { journal_path } => journal_path,
        Commands::Ledger { journal_path } => journal_path,
    };
    let journal = match cli::load_journal(journal_path) {
        Ok(journal) => journal,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Check { journal_path } => {
            println!("{journal_path} is valid");
        }
        Commands::Ledger { .. } => {
            dbg!(queries::transactions_for(
                &journal,
                "Equity::OpeningBalances"
            ));
        }
    }
}
