//! The one seam. Every impure interaction the application has with the outside
//! world goes through `Host`, so tests can drive the whole application through
//! [`crate::fake::FakeHost`] and assert on the commands it emitted and the
//! files it wrote.

use anyhow::{Context, Result};
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// The result of running a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    /// A successful run with the given stdout.
    pub fn ok(stdout: &str) -> Self {
        Output {
            status: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    /// A failed run with the given exit status and stderr.
    pub fn fail(status: i32, stderr: &str) -> Self {
        Output {
            status,
            stdout: String::new(),
            stderr: stderr.to_string(),
        }
    }

    pub fn succeeded(&self) -> bool {
        self.status == 0
    }

    /// stdout with the trailing newline removed.
    pub fn trimmed(&self) -> &str {
        self.stdout.trim_end_matches('\n')
    }
}

/// The user dismissed a prompt with Esc or Ctrl-C. Carried as an error so that
/// every prompt aborts with a plain `?` and unwinds having emitted nothing;
/// [`aborted`] tells it apart from a real failure at the top.
#[derive(Debug)]
pub struct Aborted;

impl fmt::Display for Aborted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("aborted")
    }
}

impl std::error::Error for Aborted {}

/// Whether an error is the user walking away from a prompt rather than a fault.
pub fn aborted(error: &anyhow::Error) -> bool {
    error.downcast_ref::<Aborted>().is_some()
}

pub trait Host {
    /// Run `argv`, optionally in `cwd`. A non-zero exit is a normal outcome
    /// reported in [`Output::status`], not an error; `Err` means the command
    /// could not be run at all.
    fn run(&self, argv: &[&str], cwd: Option<&Path>) -> Result<Output>;

    /// Read a file, reporting absence as `Ok(None)`.
    fn read_file(&self, path: &Path) -> Result<Option<String>>;

    /// Write a file, creating parent directories as needed.
    fn write_file(&self, path: &Path, contents: &str) -> Result<()>;

    fn exists(&self, path: &Path) -> bool;

    /// Directories under `root` (inclusive) to `max_depth`, not following
    /// symlinks. Unreadable directories are skipped rather than failing.
    fn walk(&self, root: &Path, max_depth: usize) -> Result<Vec<PathBuf>>;

    /// Replace this process with `argv`. Returns only on failure: handing the
    /// terminal to `tmux attach-session` is the last thing the tool does, and
    /// an attach without the real terminal behind it is not an attach.
    fn exec(&self, argv: &[&str]) -> Result<()>;

    /// The terminal's width and height in cells: what a row is laid out
    /// against, and what decides how many rows the picker may show. A terminal
    /// that will not answer is [`FALLBACK_TERMINAL`].
    fn terminal(&self) -> (usize, usize);

    /// Choose one of `options`, returning its index. Typing filters the list.
    fn select(&self, message: &str, options: &[String]) -> Result<usize>;

    /// Free text, pre-filled with an editable `default`.
    fn input(&self, message: &str, default: &str) -> Result<String>;

    fn confirm(&self, message: &str, default: bool) -> Result<bool>;

    fn now(&self) -> SystemTime;

    /// Whether the tool is running inside a tmux client, which decides between
    /// switching that client and attaching a new one.
    fn in_tmux(&self) -> bool;

    /// The user's home directory, below which config and state live.
    fn home(&self) -> Result<PathBuf>;
}

/// The one production implementation.
pub struct RealHost;

impl Host for RealHost {
    fn run(&self, argv: &[&str], cwd: Option<&Path>) -> Result<Output> {
        let (bin, args) = argv.split_first().context("run called with empty argv")?;
        let mut cmd = Command::new(bin);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let out = cmd
            .output()
            .with_context(|| format!("running `{}`", argv.join(" ")))?;
        Ok(Output {
            status: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    fn read_file(&self, path: &Path) -> Result<Option<String>> {
        match std::fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    fn write_file(&self, path: &Path, contents: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn walk(&self, root: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
        // Returns every directory in range. Pruning — stopping at the first
        // `.git`, skipping `node_modules` — is the scanner's job in ticket 05,
        // which may want a prune predicate here rather than filtering after.
        Ok(walkdir::WalkDir::new(root)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_dir())
            .map(|entry| entry.into_path())
            .collect())
    }

    fn exec(&self, argv: &[&str]) -> Result<()> {
        use std::os::unix::process::CommandExt;
        let (bin, args) = argv.split_first().context("exec called with empty argv")?;
        // `exec` returns only when it failed to replace the process.
        Err(anyhow::Error::new(Command::new(bin).args(args).exec()))
            .with_context(|| format!("running `{}`", argv.join(" ")))
    }

    fn terminal(&self) -> (usize, usize) {
        crossterm::terminal::size()
            .map(|(width, height)| (width as usize, height as usize))
            .unwrap_or(FALLBACK_TERMINAL)
    }

    fn select(&self, message: &str, options: &[String]) -> Result<usize> {
        // inquire's default scorer is a skim fuzzy match, so `vbsn3` finds
        // `ada/VBSN-3-tmux` and no external `fzf` is needed.
        prompt(
            inquire::Select::new(message, options.to_vec())
                .with_page_size(page_size(self.terminal().1))
                .with_help_message(HELP)
                .with_render_config(render_config())
                .raw_prompt(),
        )
        .map(|choice| choice.index)
    }

    fn input(&self, message: &str, default: &str) -> Result<String> {
        prompt(
            inquire::Text::new(message)
                .with_initial_value(default)
                .prompt(),
        )
    }

    fn confirm(&self, message: &str, default: bool) -> Result<bool> {
        prompt(
            inquire::Confirm::new(message)
                .with_default(default)
                .prompt(),
        )
    }

    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn in_tmux(&self) -> bool {
        std::env::var_os("TMUX").is_some_and(|value| !value.is_empty())
    }

    fn home(&self) -> Result<PathBuf> {
        // Constitution §7: Unix only, so `$HOME` is the whole answer and a
        // crate would be a dependency for one environment variable.
        let home = std::env::var("HOME").context("HOME is not set")?;
        Ok(PathBuf::from(home))
    }
}

/// What a terminal that will not answer is taken to be. Eighty by
/// twenty-four is the oldest safe answer and the one every pipe deserves.
pub const FALLBACK_TERMINAL: (usize, usize) = (80, 24);

/// One line, under every prompt, saying what the keys do.
const HELP: &str = "↑↓ move · type to filter · enter select · esc cancel";

/// How many rows the picker may show: the terminal less the prompt line, the
/// help line and two rows of margin, so opening the list never scrolls the
/// prompt off its own screen. Floored at five — a very short terminal still
/// needs enough rows to be a list.
fn page_size(height: usize) -> usize {
    height.saturating_sub(4).max(5)
}

/// The default configuration — which honours `NO_COLOR` — with a sharper
/// cursor: `❯` reads as a cursor at a glance where `>` reads as text.
fn render_config() -> inquire::ui::RenderConfig<'static> {
    use inquire::ui::{Color, RenderConfig, Styled};
    RenderConfig::default()
        .with_highlighted_option_prefix(Styled::new("❯").with_fg(Color::LightCyan))
}

/// inquire reports Esc and Ctrl-C as errors; here they are an abort.
fn prompt<T>(result: inquire::error::InquireResult<T>) -> Result<T> {
    use inquire::InquireError::{OperationCanceled, OperationInterrupted};
    match result {
        Err(OperationCanceled | OperationInterrupted) => Err(Aborted.into()),
        other => Ok(other?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_fills_the_terminal_but_never_overruns_it() {
        assert_eq!(page_size(50), 46, "a tall terminal shows 46 rows, not 7");
        assert_eq!(page_size(24), 20);
        assert_eq!(
            page_size(8),
            5,
            "floored, so a short terminal is still a list"
        );
        assert_eq!(
            page_size(0),
            5,
            "a terminal with no height does not underflow"
        );
    }

    #[test]
    fn a_terminal_that_will_not_answer_is_eighty_by_twenty_four() {
        let (width, height) = RealHost.terminal();

        assert!(width > 0 && height > 0, "{width}x{height}");
        // Tests run without a controlling terminal, so this is the fallback.
        assert_eq!((width, height), FALLBACK_TERMINAL);
    }

    /// A throwaway directory tree: `root/a/b/c`, plus a symlink `root/link`
    /// pointing back at `root/a`.
    fn tree(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vibestation-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a/b/c")).unwrap();
        std::os::unix::fs::symlink(root.join("a"), root.join("link")).unwrap();
        root
    }

    #[test]
    fn walk_is_depth_bounded_and_does_not_follow_symlinks() {
        let root = tree("walk");
        let dirs = |depth| {
            let mut d = RealHost.walk(&root, depth).unwrap();
            d.sort();
            d.iter()
                .map(|p| p.strip_prefix(&root).unwrap().display().to_string())
                .collect::<Vec<_>>()
        };

        assert_eq!(dirs(2), ["", "a", "a/b"]);
        assert_eq!(
            dirs(10),
            ["", "a", "a/b", "a/b/c"],
            "the symlink is not walked into"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_file_reads_as_absent_and_writing_creates_parents() {
        let root = tree("io");
        let nested = root.join("x/y/config.toml");

        assert_eq!(RealHost.read_file(&nested).unwrap(), None);
        assert!(!RealHost.exists(&nested));

        RealHost.write_file(&nested, "hello").unwrap();

        assert_eq!(
            RealHost.read_file(&nested).unwrap().as_deref(),
            Some("hello")
        );
        assert!(RealHost.exists(&nested));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_failing_command_is_an_output_not_an_error() {
        let out = RealHost
            .run(&["sh", "-c", "echo out; echo err >&2; exit 3"], None)
            .unwrap();

        assert_eq!(out.status, 3);
        assert_eq!(out.trimmed(), "out");
        assert_eq!(out.stderr.trim(), "err");
        assert!(!out.succeeded());
    }

    #[test]
    fn run_uses_the_given_working_directory() {
        let root = tree("cwd");
        let out = RealHost.run(&["pwd"], Some(&root.join("a"))).unwrap();

        assert_eq!(out.trimmed(), root.join("a").to_str().unwrap());

        std::fs::remove_dir_all(&root).unwrap();
    }
}
