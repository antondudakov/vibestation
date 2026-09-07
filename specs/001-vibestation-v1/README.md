# Spec 001 — Vibestation v1: tickets

Tracer-bullet slices of [`../001-vibestation-v1.md`](../001-vibestation-v1.md).
Each ticket is sized for a single fresh context window and leaves the tool in a
working, installable, tested state, per constitution §6.

Work the **frontier**: any ticket whose blockers are all done.

| # | Ticket | Blocked by |
|---|--------|-----------|
| 01 | [Cargo skeleton, `Host` seam and CI](01-skeleton-host-seam-ci.md) | — |
| 02 | [Config file and first-run prompt](02-config-and-first-run.md) | 01 |
| 03 | [tmux session listing](03-tmux-session-listing.md) | 01 |
| 04 | [Session picker, attach vs. switch-client](04-session-picker-attach.md) | 03 |
| 05 | [Project scanner and cache](05-project-scanner.md) | 02 |
| 06 | [Worktree grouping](06-worktree-grouping.md) | 05 |
| 07 | [Frecency ranking and the unified picker](07-frecency-unified-picker.md) | 04, 06 |
| 08 | [Name suggestion, session creation, resume path](08-naming-session-creation.md) | 07 |
| 09 | [Default-branch detection, fetch, in-place branching](09-default-branch-fetch-in-place.md) | 08 |
| 10 | [Worktree creation and the three-way strategy choice](10-worktree-creation.md) | 09 |
| 11 | [Refresh and add-manually picker actions](11-picker-actions.md) | 07 |
| 12 | [Release workflow and Homebrew tap](12-release-workflow-and-tap.md) | 01 |
| 13 | [`--help` and README](13-help-and-readme.md) | 10, 11, 12 |

Critical path: 01 → 02 → 05 → 06 → 07 → 08 → 09 → 10 → 13.
Runs in parallel: 03 → 04 alongside 02 → 05 → 06; 11 alongside 08 → 10; 12 from
the moment 01 lands.

Two milestones worth noting: **04** is the first genuinely useful version — a
working session switcher. **07** is the picker as the spec describes it.
