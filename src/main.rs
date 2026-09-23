use clap::Parser;
use vibestation::host::{aborted, RealHost};

/// One picker for your tmux sessions and git projects.
#[derive(Parser)]
#[command(name = "vibestation", version = VERSION, about, long_about = LONG_ABOUT)]
struct Cli {}

/// `--help`. There are no flags worth documenting, so what the help has to
/// explain is the tool: one picker, and what each kind of row does when you
/// pick it. `-h` still shows the one-line about above.
const LONG_ABOUT: &str = "\
One picker for your tmux sessions and git projects.

Opens a single fuzzy picker: your live tmux sessions first, then your git
projects ranked by how often and how recently you open them, each with its
worktrees indented beneath it. Typing filters every row at once, so a ticket
identifier reaches its worktree directly.

Choosing a session joins it — switching the client when you are already inside
tmux, attaching otherwise.

Choosing a project or a worktree starts work: it settles on a session name
(from the branch you are on, or from one line like `VBSN-1 initialize the
project`), offers to cut that branch in a new worktree beside the checkout or
in place, creates the session in whichever directory the work now lives in, and
drops you into it.

Left arrow rescans your projects directories. Right arrow opens a menu for the
row under the cursor: kill or rename a session, open a project or worktree in
$EDITOR, remove a worktree. The last row adds a repository by path.

Configuration is ~/.vibestation/config.toml, written on first run, which asks
one question.";

/// `0.1.0 (build 43)` for a binary from `scripts/build.sh`, plain `0.1.0` for
/// a local `cargo build`. The build number is the commit count, stamped in at
/// compile time, so a shipped binary can be traced to the commit that made it —
/// which its own commit hash cannot do, since the binary is part of that commit.
///
/// Composed by the build script rather than here because clap wants a
/// `&'static str` and a const cannot format one.
const VERSION: &str = match option_env!("VIBESTATION_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

fn main() {
    Cli::parse();
    if let Err(e) = vibestation::run(&RealHost) {
        // Dismissing the picker is not a failure: it costs nothing and says so.
        if aborted(&e) {
            return;
        }
        eprintln!("vibestation: {e:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_help_says_what_the_tool_does_and_which_version_it_is() {
        Cli::command().debug_assert();
        let help = Cli::command().render_long_help().to_string();

        assert!(help.contains("tmux sessions"), "{help}");
        assert!(help.contains("worktree"), "{help}");
        assert!(
            help.contains("config.toml"),
            "the help points at the config file: {help}"
        );
        assert_eq!(Cli::command().get_version(), Some(VERSION));
    }
}
