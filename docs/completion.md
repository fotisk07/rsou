# Shell completion

Account completion is available for `ledger` and both account arguments of
`post`, alongside command and option completion. Suggestions come from the
journal each time you press Tab.

Install the current binary:

```sh
cargo install --path . --force
```

## Zsh

Add this to `~/.zshrc`, after your existing `compinit` setup:

```zsh
source <(COMPLETE=zsh rsou)
```

If you do not already initialize completion, use:

```zsh
autoload -Uz compinit
compinit
source <(COMPLETE=zsh rsou)
```

## Bash

Add this to `~/.bashrc`:

```bash
source <(COMPLETE=bash rsou)
```

Account paths use `/`, so both shells use Clap's standard completion integration.
No custom Bash adapter, word-break changes, or bash-completion package is needed.

Open a new shell or run the appropriate setup line in the current shell.

## Usage

```sh
rsou ledger Assets/<TAB>
rsou post 10 Assets/<TAB>
rsou post 10 Assets/Bank Expenses/<TAB>
```

By default, suggestions use `journal.sou` in the current directory. For another
journal, put `-j PATH` (or `--journal-path PATH`) before the argument you are
completing:

```sh
rsou -j personal.sou ledger Assets/<TAB>
rsou ledger -j personal.sou Assets/<TAB>
```

Missing or invalid journals produce no account suggestions and no error
messages. Completion never writes to the journal.

During development, you can skip installation and initialize completion with
`source <(COMPLETE=zsh ./target/debug/rsou)` (or `COMPLETE=bash`), then invoke
`./target/debug/rsou` instead of `rsou`.
