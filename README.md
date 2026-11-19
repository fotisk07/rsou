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
