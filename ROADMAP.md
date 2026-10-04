# rsou roadmap

## Scope

A CLI for a human-editable accounting journal, rebuilt from Sou in Rust:

- Post transactions.
- Show transactions associated with an account.
- Show profit and loss (P&L) and a balance sheet.

The journal file is the source of truth. Manual edits must remain supported.
Recurring transactions are out of scope for now.

## How to use this roadmap

Keep future milestones broad. Expand only the current milestone into concrete
implementation steps as its decisions become clear. A milestone is complete
when its observable completion criteria are met, not merely when its code exists.
Update this document and commit it alongside the relevant code changes.

## Completed

- [x] Journal data types and account-tree lookup using `::` paths.
- [x] Journal parsing with messages and line numbers for parse errors.
- [x] Separate accounting validation: known accounts, balanced transactions,
      and at least two postings.
- [x] CLI `check` command and shared read → parse → validate pipeline.
- [x] Account ledger including direct and descendant postings, chronological
      ordering, running balances, and table output.

## Current milestone: Posting transactions

### Goal

Record a transaction through the CLI and immediately see it in the ledger.
Start with a basic two-account transaction; broader split-posting options can follow.

### Done when

- [ ] The CLI accepts the amount, source account, destination account,
      description, and date.
- [ ] A valid transaction is appended and reloads correctly.
- [ ] Invalid input leaves the journal file unchanged.
- [ ] Existing manually written contents and formatting are preserved.
- [ ] The new transaction appears with the correct amounts in the account ledger.
- [ ] Tests cover successful posting and rejection without file modification.

### Decisions to make

- CLI syntax and amount/sign conventions.
- Default date and explicit date syntax.
- Transaction text format, including separation from existing contents.
- File-writing safety and handling write failures.

### Implementation steps

To be expanded when starting this milestone. Keep steps small and explain any
new Rust constructs as they are introduced. Avoid unrelated restructuring.

## Upcoming milestones

### P&L

Calculate income, expenses, and the net result over a date range.
Support sensible date defaults and depth-limited account display, including
postings directly to parents as well as their descendants.

### Balance sheet

Calculate balances as of a date, including assets, liabilities, equity, and
the current result. Support sensible defaults and depth-limited display.
Verify that the accounting equation holds on hand-checkable examples.

### Polish

- Ledger date ranges and opening balances.
- More flexible split posting.
- Consistent CLI options and output.
- Parser edge cases and file-writing safety improvements.

## Development approach

Finish one milestone before moving to another. Agree on its behavior, implement
small steps, test them, and commit the completed capability. Sou is the behavior
reference, not an architecture that must be copied. Add abstractions and files
only when they clarify an existing responsibility.
