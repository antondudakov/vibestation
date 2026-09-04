# Vibestation Constitution

Principles that constrain every decision in this repo. When a change conflicts
with one of these, the change is wrong until this document is amended.

## 1. One seam

All impurity — process execution, filesystem, user prompts, the clock — lives
behind a single `Host` trait. Application logic is a pure function of `Host`.
There is exactly one production implementation and one test fake. New traits
for testability require amending this document.

**Why:** every test drives the whole application through one fake. No mock
scaffolding, no partial integration, no module that is testable only in theory.

## 2. Shell out, don't bind

`tmux` and `git` are invoked as subprocesses. No `libgit2`, no tmux control-mode
protocol bindings. The observable behaviour of this tool *is* the argv it emits.

**Why:** the CLIs are stable, universally installed, and trivially assertable in
tests. Library bindings add build complexity and a second source of truth.

## 3. No network on the hot path

Opening the picker never touches the network. Neither does creating a session or
a branch. Vibestation branches from local refs; keeping them current is the
user's job.

**Why:** a session switcher that stalls on a flaky VPN is worse than no session
switcher.

## 4. Never mutate a dirty working tree

Git state changes are opt-in, confirmed, and refuse to run against uncommitted
changes. When a mutation is refused, the tool still does the non-destructive part
of the job and says what it skipped.

**Why:** losing work to a tool that was trying to be helpful is unforgivable, and
this tool runs at the moment you are most distracted.

## 5. Config is one question deep

First run asks exactly one question. Every other setting has a working default
and is discoverable by reading the generated file. New settings must justify
their existence against a default that would have been fine.

## 6. Every PR is shippable and tested

Each PR leaves the tool in a working, installable state and carries tests for the
logic it adds. No PR exists solely to scaffold for a later one.

## 7. Unix only, for now

macOS and Linux. Windows support is not a goal and no code is written to
accommodate it. `$HOME` is read directly; paths are POSIX.

## 8. Few, boring dependencies

A dependency must save more code than it costs to understand. Prefer the standard
library. The dependency list is reviewed at every addition, not accumulated.
