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

- [x] The CLI accepts the amount, source account, destination account,
      description, and date.
- [x] A valid transaction is recorded and reloads correctly.
- [x] Invalid input leaves the journal file unchanged.
- [x] Writing normalizes the journal to a canonical format while preserving
      accounting data and sibling account order; root accounts use a fixed order.
- [x] Posting appends transactions in file order without reordering existing ones.
- [x] An explicit `sort [-j PATH]` command orders transactions chronologically,
      preserving their relative order on the same date.
- [x] The new transaction appears with the correct amounts in the account ledger.
- [x] CLI checks cover successful posting and rejection without file modification.

### Agreed design

- Rewrite the whole journal in canonical format; manual edits remain supported.
- Posting preserves transaction order; only the explicit `sort` command reorders
  transactions, preserving same-date transaction order.
- Preserve required `name`, `year`, and `currency` metadata; year is informational.
- Use fixed root-account order and preserve sibling account order.
- Use two-space indentation, two spaces between posting account and amount,
  exactly two decimal places, blank lines between sections and transactions,
  and a final newline.
- Keep formatting separate from file I/O; validate before safely replacing the
  journal using a same-directory temporary file.

### CLI behavior

- `post AMOUNT FROM_ACCOUNT TO_ACCOUNT DESCRIPTION [-d YYYY-MM-DD]`.
- Amount must be positive and exactly representable with two decimal places;
  source receives `-amount`, destination receives `+amount`.
- Date defaults to today; descriptions must be nonempty and single-line.

### Implementation steps

The user implements the code with guidance. Keep steps small and avoid unrelated
restructuring.

1. [x] Agree on the canonical journal format.
2. [x] Add stable transaction sorting and required metadata parsing.
3. [x] Format the complete journal; verify round trips preserve accounting data.
4. [x] Implement same-directory temporary-file replacement (dedicated write
       tests deferred).
5. [x] Finish CLI posting and verify successful posting, unchanged files on
       rejection, and correct ledger amounts.

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
