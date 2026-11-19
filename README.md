# Sou

A personal accounting CLI written in Rust.

## Install

With Rust and Cargo installed, run from this repository:

```sh
cargo install --path .
sou --help
```

Cargo installs `sou` into `~/.cargo/bin` by default. Ensure that directory is on
`PATH`. Run `cargo install --path .` again after pulling updates to rebuild and
replace the installed executable.

## Full-year reports

Use the journal's full year, regardless of today's date:

```sh
sou pnl --year                 # January 1 through December 31
sou bs --year                  # Balance sheet as of December 31
sou pnl --year --by-month      # Full year in monthly columns
sou pnl --year --by-quarter    # Full year in quarterly columns
```

Omit the number from `--month` (`-m`) or `--quarter` (`-q`) to select the
current whole month or quarter (current-year journals only). Explicit numbers
still select a period in any journal year:

```sh
sou pnl --month
sou pnl --quarter
sou bs --month                # Current month-end
sou pnl --month 3
sou pnl --quarter 2
```

## Date selection

All explicit dates use `DD-MM` in the journal year, with inclusive bounds.

- `pnl` and `ledger`: `--from` / `--to` select a range.
- `bs` and `recur post`: `--to` selects a cutoff. Both accept `--year`,
  `--month [1-12]`, `--quarter [1-4]`, `--week`, and `--today`.
- Bare `--month` / `--quarter`, `--week`, and `--today` require a current-year
  journal. `--week` means through Sunday, capped at December 31.
- `bs` defaults to current month-end (year-end for other journal years).
  `recur post` defaults to today; other-year journals need an explicit cutoff.
- `recur add --start DD-MM` sets the first occurrence. Omitting `--start`
  defaults to today only for current-year journals.

The former cutoff flags `--at`, `--date`, and `-d` remain accepted as aliases
for `--to` on `bs` and `recur post`.

```sh
sou bs --to 30-06
sou recur post --to 30-06 --dry-run
sou recur post --month 6 --dry-run
```

## Account completion

Enable Tab completion by adding this to `~/.zshrc` (zsh):

```sh
autoload -Uz compinit
compinit
source <(COMPLETE=zsh sou)
```

For bash, add `source <(COMPLETE=bash sou)` to `~/.bashrc` instead.
Restart your shell, then press Tab while typing an account path, for example
`sou ledger Assets/`. Accounts are read from `journal.sou` in the current
directory; for another journal, put `-j path/to/journal.sou` before the command.
