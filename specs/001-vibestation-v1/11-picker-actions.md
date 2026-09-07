# 11: Refresh and add-manually picker actions

**What to build:** The picker's two escape hatches. A developer who just cloned a
repository picks "refresh" and it appears, without editing config or clearing a
cache by hand. A developer whose repository lives outside their configured
projects directories picks "add manually", gives a path, and it is validated and
written into their config so it persists across refreshes and they can see and
edit it later.

**Blocked by:** 07.

**Status:** ready-for-agent

- [ ] Refresh and add-manually appear as entries in the picker
- [ ] Refresh rescans the configured roots and rewrites the cache
- [ ] Add-manually rejects a path that is not a git repository
- [ ] An accepted path appears in the config file's extra projects, with the rest of the file preserved
- [ ] A manually added project appears in the picker and survives a subsequent refresh
