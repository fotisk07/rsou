mod models;
mod parser;
use std::fs;

fn main() {
    let file_path = "test_data/simple.sou";
    let contents = fs::read_to_string(file_path).expect("Should have been able to read the file");

    let journal = parser::parse_journal(&contents);
    dbg!(journal);
}
