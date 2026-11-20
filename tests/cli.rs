use std::process::Command;

fn journal_file() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("journal.sou");
    std::fs::write(
        &path,
        "[JOURNAL]
name: Test
year: 2024
currency: EUR
[ACCOUNTS]
Assets
  Bank
Expenses
  Food
[ALIASES]
bank = Assets/Bank
food = Expenses/Food
[TRANSACTIONS]
",
    )
    .unwrap();
    (directory, path)
}

#[test]
fn posting_saves_exact_transfers_and_rejects_fractional_cents_without_writing() {
    let (_directory, path) = journal_file();
    let post = |amount| {
        Command::new(env!("CARGO_BIN_EXE_sou"))
            .arg("-j")
            .arg(&path)
            .args(["post", amount, "bank", "food", "Meal", "--date", "29-02"])
            .output()
            .unwrap()
    };
    for amount in ["12.34", "-2.34"] {
        let output = post(amount);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("2024-02-29 Meal\n  Assets/Bank  -12.34\n  Expenses/Food  12.34"));
    assert!(contents.contains("2024-02-29 Meal\n  Assets/Bank  2.34\n  Expenses/Food  -2.34"));

    let output = post("12.345");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("two decimal places"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
}

#[test]
fn ledger_alias_matches_canonical_account_report() {
    let (_directory, path) = journal_file();
    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_sou"))
            .arg("-j")
            .arg(&path)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    run(&[
        "post",
        "12.34",
        "Assets/Bank",
        "Expenses/Food",
        "Meal",
        "--date",
        "29-02",
    ]);
    let report = run(&["ledger", "bank", "--year"]);
    assert_eq!(report, run(&["ledger", "Assets/Bank", "--year"]));
    assert!(report.starts_with("Ledger | Assets/Bank |"));
    assert!(report.contains("Meal"));
}

#[test]
fn invalid_splits_fail_without_changing_the_journal() {
    let (_directory, path) = journal_file();
    let contents = std::fs::read_to_string(&path).unwrap();

    for (postings, expected_error) in [
        (
            ["Assets/Bank", "-10", "Expenses/Food", "9.99"],
            "unbalanced",
        ),
        (
            ["Assets/Bank", "-10", "Expenses/Missing", "10"],
            "unknown account",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_sou"))
            .arg("-j")
            .arg(&path)
            .args(["split", "Meal"])
            .args(postings)
            .args(["--date", "29-02"])
            .output()
            .unwrap();

        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains(expected_error), "{error}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), contents);
    }
}
