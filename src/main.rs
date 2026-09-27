mod models;
mod parser;
mod validation;
use clap::{Parser, Subcommand};
use std::fs;

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
}
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { journal_path } => {
            let contents = match fs::read_to_string(&journal_path) {
                Ok(contents) => contents,
                Err(error) => {
                    eprintln!("cannot read {journal_path}: {error}");
                    std::process::exit(1);
                }
            };
            let journal = match parser::parse_journal(&contents) {
                Ok(journal) => journal,
                Err(error) => {
                    match error.line {
                        Some(line) => eprintln!("line {line}: {}", error.message),
                        None => eprintln!("{}", error.message),
                    }
                    std::process::exit(1);
                }
            };

            if let Err(error) = validation::validate_journal(&journal) {
                eprintln!("invalid journal: {error:?}");
                std::process::exit(1);
            }

            println!("{} is valid", &journal_path)
        }
    }
}
