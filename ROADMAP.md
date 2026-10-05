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
- [ ] A valid transaction is recorded and reloads correctly.
- [ ] Invalid input leaves the journal file unchanged.
- [ ] Writing normalizes the journal to a canonical format while preserving
      accounting data and sibling account order; root accounts use a fixed order.
- [ ] Transactions are written in chronological order, preserving their relative
      order on the same date.
- [ ] The new transaction appears with the correct amounts in the account ledger.
- [ ] Tests cover successful posting and rejection without file modification.

### Agreed architecture

- Parse without changing file order; sort transactions explicitly before writing.
- Rewrite the complete journal in a canonical format rather than preserving
  handwritten whitespace. Manual edits remain supported on the next read.
- Use a stable date sort so same-date transactions retain their relative order.
- Separate pure journal formatting from filesystem writes.
- Validate before writing, then replace the journal using a temporary file in
  the same directory. Never delete the original before replacement.
- A standalone `format` command is optional and must not delay posting.

### Decisions to make

- CLI syntax and amount/sign conventions.
- Default date and explicit date syntax.
- Canonical account and transaction text format, including the header metadata
  currently ignored by the parser and a fixed root-account order.
- Handling comments or other text not represented in the journal model:
  inspect parser behavior before implementing full rewriting.
- File-writing safety, permissions, concurrent edits, and handling write failures.

### Implementation steps

Keep steps small and explain any new Rust constructs as they are introduced.
Avoid unrelated restructuring; the user will implement the code with guidance.

1. [ ] Inspect the parser and fixtures; agree on the canonical format and what
       information rewriting retains or discards.
2. [ ] Add explicit stable transaction sorting, with tests for out-of-order
       dates and unchanged relative order on the same date.
3. [ ] Add pure journal formatting, preserving sibling account order. Test parse/format
       round trips and formatting idempotence.
4. [ ] Add safe file replacement using a same-directory temporary file, with
       tests for successful replacement and failures leaving the original intact.
5. [ ] Complete CLI input and transaction construction/validation, including
       description and date. Reuse sorting, formatting, and safe writing.
6. [ ] Test successful posting, rejection without modification, backdated
       posting, and correct amounts in the ledger.
7. [ ] Optionally expose the same sorting/formatting/writing pipeline through
       a `format` command.

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
