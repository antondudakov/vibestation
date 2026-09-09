# 02: Config file and first-run prompt

**What to build:** On a machine that has never run vibestation, the tool asks
exactly one question — where the developer's projects live — and writes a config
file with every other setting defaulted and commented so the developer can
discover them later. On every subsequent run the config is loaded silently. A
config file the developer has broken by hand produces an error that names the
file rather than a parse trace.

**Blocked by:** 01.

**Status:** done

- [x] First run asks one question only: the projects directory
- [x] A TOML config is written at a predictable path under the user's home, with explanatory comments on every field
- [x] Fields present: projects dirs (a list, even though first run fills one entry), extra projects, username, optional default-branch override, scan depth (default 10), fetch-before-branch (default true)
- [x] Username is defaulted from `git config user.name`, slugified
- [x] Home directory comes from the `HOME` environment variable, not a crate
- [x] A hand-written config with fields omitted loads, with defaults filling the gaps
- [x] A malformed config produces a clear error naming the file
- [x] A written config round-trips through load
