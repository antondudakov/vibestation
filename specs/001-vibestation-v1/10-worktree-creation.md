# 10: Worktree creation and the three-way strategy choice

**What to build:** The full branching choice from the spec. The developer is
asked how to create the branch: a new worktree beside the main checkout (the
default, because it alters no existing checkout and is therefore always safe),
in place in the existing checkout, or neither. A dirty main checkout removes the
in-place option but leaves the worktree option untouched, so work in progress no
longer blocks starting something new.

The worktree lands as a sibling directory named for the project and the branch,
so it is identifiable from a shell prompt and every other tool sees it as an
ordinary directory. Repeating the action is harmless: an existing worktree on
that branch is reused. A path occupied by something else stops the operation —
vibestation never writes into a directory it did not create.

**Blocked by:** 09.

**Status:** ready-for-agent

- [ ] The strategy choice is three-way, with the new worktree pre-selected as the default
- [ ] With a dirty checkout the in-place option is withheld while the worktree option remains available
- [ ] The worktree directory name is the main checkout's directory name joined to the branch with the `username/` prefix stripped and remaining slashes replaced by hyphens
- [ ] Worktree creation and branch creation are a single git operation, emitted with the expected branch, derived sibling path and base ref
- [ ] A worktree that already exists for that branch at that path is reused, with no add emitted
- [ ] A path occupied by anything else stops the operation with a message naming the path, emitting no add and creating no session
- [ ] A session is created rooted in the directory the work now lives in — the new or reused worktree, or the main checkout for in-place and neither
- [ ] No strategy prompt appears on the resume path
