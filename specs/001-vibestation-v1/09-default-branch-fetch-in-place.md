# 09: Default-branch detection, fetch, and in-place branching

**What to build:** A developer starting ticketed work is no longer left sitting
on the default branch. After the name is accepted, they are offered a branch —
created in place in the existing checkout, or not at all. Before it is cut, they
are offered a `git fetch origin` so the branch starts from a current
`origin/<default>` rather than a `main` that is four days stale; the answer is
pre-selected from config so "always fetch" can be made the default and stop
being a question.

A fetch that fails — offline, VPN down, no remote, expired credentials — warns
and cuts from the local ref instead. It never aborts the work. The branch is cut
from `origin/<default>` rather than a fast-forwarded local ref, so the local
default branch is never touched and the operation cannot fail on divergence.

If the checkout has uncommitted changes the in-place option is withheld with an
explanation, not offered and then refused.

**Blocked by:** 08.

**Status:** done

- [x] The default branch is detected through the fallback chain `origin/HEAD`, then `main`, then `master`, each stage exercised
- [x] A config override for the default branch takes precedence, so a repository on `develop` works
- [x] A branch is offered only when the accepted session name differs from the current branch — never on the resume path or after selecting an existing worktree
- [x] Declining the branch still creates the session on the current branch and mutates no git state
- [x] A fetch confirmation precedes branch creation, pre-answered from config; declining emits no fetch
- [x] On fetch success the branch is cut from `origin/<default>`; the local default branch ref is never modified
- [x] A failing fetch produces a warning and a branch cut from the local ref rather than an abort
- [x] No fetch is emitted anywhere on the path to an existing session
- [x] With uncommitted changes the in-place option is withheld with an explanation
