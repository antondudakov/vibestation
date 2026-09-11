# Spec 002 — A picker you can read: tickets

Slices of [`../002-picker-legibility.md`](../002-picker-legibility.md). Each
ticket is sized for a single fresh context window and leaves the tool in a
working, installable, tested state, per constitution §6.

| # | Ticket | Blocked by |
|---|--------|-----------|
| 01 | [Terminal size behind the seam, and a list that fills it](01-terminal-size-and-page.md) | — |
| 02 | [One table, fitted to the terminal](02-one-fitted-table.md) | 01 |
| 03 | [What tmux knows: order, age, and which session is yours](03-session-age-and-here.md) | 02 |
| 04 | [The prompt line and the README](04-prompt-line-and-docs.md) | 03 |

Strictly sequential: each ticket needs the one before it. **01** is already worth
shipping on its own — a list seven rows tall becomes a list the height of the
terminal. **02** is the spec's substance.
