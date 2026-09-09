# 05: Project scanner and cache

**What to build:** The developer's repositories are discovered wherever they
keep them. The tool walks the configured projects directories, finds git
repositories nested several levels deep, merges in any manually configured extra
projects, resolves display names so two same-named directories in different roots
are distinguishable, and writes the result to a cache so opening the picker is
instant rather than rescanning the disk.

**Blocked by:** 02.

**Status:** done

- [x] More than one projects directory can be configured and all are scanned
- [x] The walk stops descending at the first `.git` it finds, so submodules and vendored repositories are not listed
- [x] The walk respects the configured depth cap
- [x] Symlinks are skipped, so the scan cannot loop or wander outside the configured roots
- [x] Manually configured extra projects are merged in and deduplicated against scan results
- [x] Two same-named directories in different roots each show their parent directory to disambiguate; unambiguous names stay bare
- [x] The discovered list is written to a JSON cache under the vibestation directory
- [x] The scan performs no network access
