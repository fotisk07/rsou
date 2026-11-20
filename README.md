# Sou

A personal accounting CLI written in Rust.

Pitch: Basic accounting, in your terminal, in a plain text journal.
Anti-pitch: Not a budgeting app or a dashboard. Other tools can do that.

## Installation

To use sou, clone this github repo and run

```sh
cargo install --path .
```

This installs sou in `~/.cargo/bin`. Make sure it is in your path.

## Quick start

To create your own `journal.sou`

```sh
sou init
```

You can create your accounts using `add` and post with `post`.

```sh
sou add Assets/Bank
sou add Expenses/Food
sou add Expenses/Household
sou post 10 Assets/Bank Expenses/Food Coffee and lunch
```

You can do more complicated postings using `split`. The amounts must sum to zero.

```sh
sou split "Grocery run" Assets/Bank -50 Expenses/Food 40 Expenses/Household 10
```

To inspect your accounts use `list` and `ledger`, and to show your P&L and balance sheet use `pnl` and `bs`.

```sh
sou list
sou ledger Assets/Bank
sou pnl
sou bs
```

You can choose the timeframe and how fine you want your P&L to be.

```sh
sou pnl --month
sou pnl --quarter 2
sou pnl --from 01-03 --to 15-06
sou pnl --year --by-month --depth 2
sou bs --to 30-06
```

Dates are `DD-MM`, in the journal year. Use `sou <command> --help` for the rest.

Export P&L reports to an Excel/LibreOffice workbook, preserving account indentation and numeric amounts:

```sh
sou pnl --year --depth 2 --format xlsx > pnl.xlsx
sou pnl --year --by-month --depth 2 --format xlsx > monthly.xlsx
```

The same date filters and account depth apply. Bold parent rows are subtotals, not additional amounts to sum.

For plain CSV data instead:

```sh
sou pnl --year --depth 2 --format csv > pnl.csv
sou pnl --year --by-month --depth 2 --format csv > monthly.csv
```

CSV includes full account paths, depth, and row type (`account`, `subtotal`, `direct`, or `result`). Subtotals include their children; do not sum all rows together. Amounts are plain decimals without thousands separators.

## Recurring postings

Create a template, then post what is due. Adding a template does not post anything.

```sh
sou recur add Lunch Assets/Bank -10 Expenses/Food 10 --week
sou recur list
sou recur post --dry-run
sou recur post
```

Monthly by default. You can also use `--day`, `--week` or `--biweekly`.
Use `--start DD-MM` to choose the first occurrence, otherwise it starts today in a current-year journal.

## Account completion

Enable Tab completion by adding this to `~/.zshrc` (zsh):

```sh
autoload -Uz compinit
compinit
source <(COMPLETE=zsh sou)
```

For bash, add `source <(COMPLETE=bash sou)` to `~/.bashrc` instead.

Accounts come from `journal.sou` in the current directory. For another journal, put `-j path/to/journal.sou` before the command.
