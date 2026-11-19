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
