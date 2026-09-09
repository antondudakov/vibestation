//! The one test fake, and with it the whole testing convention: script a
//! virtual filesystem, a map of commands to their output and a queue of
//! answers, run the application, then assert on the command log and the file
//! writes. Those two are the tool's entire observable effect on the world.

use crate::host::{Aborted, Host, Output};
use anyhow::Result;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// One scripted answer to a prompt. The variant must match the prompt the
/// application reaches, or the fake panics naming the mismatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Index into the options offered by [`Host::select`].
    Select(usize),
    Text(String),
    Confirm(bool),
    /// Esc or Ctrl-C at any prompt.
    Abort,
}

impl Answer {
    pub fn text(s: &str) -> Self {
        Answer::Text(s.to_string())
    }
}

pub struct FakeHost {
    files: RefCell<BTreeMap<PathBuf, String>>,
    commands: BTreeMap<String, Output>,
    answers: RefCell<VecDeque<Answer>>,
    now: SystemTime,
    home: PathBuf,
    in_tmux: bool,
    log: RefCell<Vec<String>>,
    writes: RefCell<Vec<(PathBuf, String)>>,
    prompts: RefCell<Vec<String>>,
}

impl Default for FakeHost {
    fn default() -> Self {
        FakeHost::new()
    }
}

impl FakeHost {
    pub fn new() -> Self {
        FakeHost {
            files: RefCell::new(BTreeMap::new()),
            commands: BTreeMap::new(),
            answers: RefCell::new(VecDeque::new()),
            now: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            home: PathBuf::from("/home/dev"),
            in_tmux: false,
            log: RefCell::new(Vec::new()),
            writes: RefCell::new(Vec::new()),
            prompts: RefCell::new(Vec::new()),
        }
    }

    /// A file present in the virtual filesystem. Its parent directories are
    /// implied, which is also how [`Host::walk`] and [`Host::exists`] learn
    /// about directories.
    pub fn file(self, path: impl AsRef<Path>, contents: &str) -> Self {
        self.files
            .borrow_mut()
            .insert(path.as_ref().to_path_buf(), contents.to_string());
        self
    }

    /// Script a command by its log key: `"git status"`, or `"/repo $ git status"`
    /// to answer only when run in that working directory. A key with a working
    /// directory wins over the bare one.
    pub fn command(mut self, key: &str, output: Output) -> Self {
        self.commands.insert(key.to_string(), output);
        self
    }

    /// Script a command that succeeds with the given stdout.
    pub fn succeeds(self, key: &str, stdout: &str) -> Self {
        self.command(key, Output::ok(stdout))
    }

    /// Script a command that fails.
    pub fn fails(self, key: &str, status: i32, stderr: &str) -> Self {
        self.command(key, Output::fail(status, stderr))
    }

    pub fn answer(self, answer: Answer) -> Self {
        self.answers.borrow_mut().push_back(answer);
        self
    }

    pub fn answers(self, answers: impl IntoIterator<Item = Answer>) -> Self {
        answers.into_iter().fold(self, |host, a| host.answer(a))
    }

    pub fn now(mut self, now: SystemTime) -> Self {
        self.now = now;
        self
    }

    pub fn home(mut self, home: impl AsRef<Path>) -> Self {
        self.home = home.as_ref().to_path_buf();
        self
    }

    pub fn in_tmux(mut self, in_tmux: bool) -> Self {
        self.in_tmux = in_tmux;
        self
    }

    /// Every command run, in order, in the same form as the scripting keys.
    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    /// Every file write, in order. A file written twice appears twice.
    pub fn writes(&self) -> Vec<(PathBuf, String)> {
        self.writes.borrow().clone()
    }

    /// The contents of a file in the virtual filesystem, including writes.
    pub fn file_contents(&self, path: impl AsRef<Path>) -> Option<String> {
        self.files.borrow().get(path.as_ref()).cloned()
    }

    /// Every prompt shown, in order, with its default: `"Ticket? [VBSN-1]"` for
    /// free text, `"Fetch? [Y/n]"` for a confirmation, the bare message for a
    /// select.
    pub fn prompts(&self) -> Vec<String> {
        self.prompts.borrow().clone()
    }

    fn key(argv: &[&str], cwd: Option<&Path>) -> String {
        match cwd {
            Some(dir) => format!("{} $ {}", dir.display(), argv.join(" ")),
            None => argv.join(" "),
        }
    }

    fn next_answer(&self, message: &str) -> Result<Answer> {
        self.prompts.borrow_mut().push(message.to_string());
        match self
            .answers
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| panic!("no scripted answer left for prompt {message:?}"))
        {
            Answer::Abort => Err(Aborted.into()),
            answered => Ok(answered),
        }
    }

    /// Directories implied by the virtual filesystem, since it stores files only.
    fn dirs(&self) -> BTreeSet<PathBuf> {
        let mut dirs = BTreeSet::new();
        for path in self.files.borrow().keys() {
            let mut dir = path.parent();
            while let Some(d) = dir {
                if !dirs.insert(d.to_path_buf()) {
                    break;
                }
                dir = d.parent();
            }
        }
        dirs
    }
}

impl Host for FakeHost {
    fn run(&self, argv: &[&str], cwd: Option<&Path>) -> Result<Output> {
        let key = FakeHost::key(argv, cwd);
        self.log.borrow_mut().push(key.clone());
        let output = self
            .commands
            .get(&key)
            .or_else(|| self.commands.get(&argv.join(" ")))
            .unwrap_or_else(|| panic!("unscripted command {key:?}; script it with .succeeds({key:?}, ..) or .fails(..)"));
        Ok(output.clone())
    }

    fn read_file(&self, path: &Path) -> Result<Option<String>> {
        Ok(self.files.borrow().get(path).cloned())
    }

    fn write_file(&self, path: &Path, contents: &str) -> Result<()> {
        self.writes
            .borrow_mut()
            .push((path.to_path_buf(), contents.to_string()));
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.borrow().contains_key(path) || self.dirs().contains(path)
    }

    fn walk(&self, root: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
        Ok(self
            .dirs()
            .into_iter()
            .filter(|dir| {
                dir.strip_prefix(root)
                    .is_ok_and(|rest| rest.components().count() <= max_depth)
            })
            .collect())
    }

    fn exec(&self, argv: &[&str]) -> Result<()> {
        // The real host never returns from this; the fake records it and lets
        // the test see that it was the last thing emitted.
        self.log.borrow_mut().push(FakeHost::key(argv, None));
        Ok(())
    }

    fn select(&self, message: &str, options: &[String]) -> Result<usize> {
        match self.next_answer(message)? {
            Answer::Select(index) => {
                assert!(
                    index < options.len(),
                    "scripted Answer::Select({index}) is out of range for {options:?}"
                );
                Ok(index)
            }
            other => panic!("prompt {message:?} is a select, but the next answer is {other:?}"),
        }
    }

    fn input(&self, message: &str, default: &str) -> Result<String> {
        match self.next_answer(&format!("{message} [{default}]"))? {
            Answer::Text(text) => Ok(text),
            other => panic!("prompt {message:?} is free text, but the next answer is {other:?}"),
        }
    }

    fn confirm(&self, message: &str, default: bool) -> Result<bool> {
        let label = format!("{message} [{}]", if default { "Y/n" } else { "y/N" });
        match self.next_answer(&label)? {
            Answer::Confirm(yes) => Ok(yes),
            other => {
                panic!("prompt {message:?} is a confirmation, but the next answer is {other:?}")
            }
        }
    }

    fn now(&self) -> SystemTime {
        self.now
    }

    fn in_tmux(&self) -> bool {
        self.in_tmux
    }

    fn home(&self) -> Result<PathBuf> {
        Ok(self.home.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> FakeHost {
        FakeHost::new()
            .file("/code/one/.git/HEAD", "")
            .file("/code/one/src/deep/nested/file.rs", "")
            .file("/elsewhere/two/.git/HEAD", "")
    }

    #[test]
    fn walk_reports_the_directories_implied_by_the_files() {
        assert_eq!(
            host().walk(Path::new("/code"), 2).unwrap(),
            [
                PathBuf::from("/code"),
                PathBuf::from("/code/one"),
                PathBuf::from("/code/one/.git"),
                PathBuf::from("/code/one/src"),
            ],
            "deeper directories and other roots are excluded"
        );
    }

    #[test]
    fn exists_covers_both_files_and_their_directories() {
        let host = host();

        assert!(host.exists(Path::new("/code/one/.git")));
        assert!(host.exists(Path::new("/code/one/.git/HEAD")));
        assert!(!host.exists(Path::new("/code/three")));
    }

    #[test]
    fn a_working_directory_key_beats_the_bare_one() {
        let host = FakeHost::new()
            .succeeds("git branch", "everywhere")
            .succeeds("/code/one $ git branch", "here");

        assert_eq!(
            host.run(&["git", "branch"], Some(Path::new("/code/one")))
                .unwrap()
                .stdout,
            "here"
        );
        assert_eq!(
            host.run(&["git", "branch"], None).unwrap().stdout,
            "everywhere"
        );
        assert_eq!(
            host.run(&["git", "branch"], Some(Path::new("/code/two")))
                .unwrap()
                .stdout,
            "everywhere",
            "an unmatched working directory falls back to the bare key"
        );
        assert_eq!(
            host.log(),
            [
                "/code/one $ git branch",
                "git branch",
                "/code/two $ git branch"
            ]
        );
    }

    #[test]
    #[should_panic(expected = "unscripted command")]
    fn an_unscripted_command_fails_loudly() {
        let _ = FakeHost::new().run(&["git", "status"], None);
    }

    #[test]
    #[should_panic(expected = "is a confirmation, but the next answer is")]
    fn a_mismatched_answer_fails_loudly() {
        let _ = FakeHost::new()
            .answer(Answer::Select(0))
            .confirm("Sure?", true);
    }

    #[test]
    fn writes_are_readable_back_and_recorded_in_order() {
        let host = FakeHost::new();

        host.write_file(Path::new("/state.json"), "first").unwrap();
        host.write_file(Path::new("/state.json"), "second").unwrap();

        assert_eq!(
            host.read_file(Path::new("/state.json")).unwrap().as_deref(),
            Some("second")
        );
        assert_eq!(
            host.writes(),
            [
                (PathBuf::from("/state.json"), "first".to_string()),
                (PathBuf::from("/state.json"), "second".to_string()),
            ]
        );
    }
}
