//! `~/.vibestation/config.toml`: load it, or ask the one first-run question and
//! write it. Every field but the projects directory is defaulted, and the file
//! is written with a comment on each so the rest is discoverable by reading it.

use crate::host::Host;
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Directories scanned for repositories. A list from the first version,
    /// though first run fills exactly one entry.
    pub projects_dirs: Vec<PathBuf>,
    /// Repositories added through the picker's add-manually action.
    pub extra_projects: Vec<PathBuf>,
    /// Prefix for generated session and branch names.
    pub username: String,
    /// Override for the branch new branches are cut from; detected when absent.
    pub default_branch: Option<String>,
    pub scan_depth: usize,
    pub fetch_before_branch: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            projects_dirs: Vec::new(),
            extra_projects: Vec::new(),
            username: String::new(),
            default_branch: None,
            scan_depth: 10,
            fetch_before_branch: true,
        }
    }
}

pub fn path(host: &dyn Host) -> Result<PathBuf> {
    Ok(host.home()?.join(".vibestation/config.toml"))
}

/// The config, asking the first-run question and writing the file when there
/// is none. Loading an existing file is silent.
pub fn load_or_init(host: &dyn Host) -> Result<Config> {
    let path = path(host)?;
    match host.read_file(&path)? {
        Some(text) => parse(&text, &path),
        None => first_run(host, &path),
    }
}

fn parse(text: &str, path: &Path) -> Result<Config> {
    toml::from_str(text).map_err(|e| {
        // The file, and where in it — not the parser's snippet and carets.
        let detail = e.to_string();
        let first = detail.lines().next().unwrap_or("invalid TOML").trim();
        anyhow!("{}: {first}", path.display())
    })
}

fn first_run(host: &dyn Host, path: &Path) -> Result<Config> {
    let home = host.home()?;
    let guess = ["code", "projects", "src", "dev"]
        .iter()
        .map(|dir| home.join(dir))
        .find(|dir| host.exists(dir))
        .unwrap_or_else(|| home.join("code"));

    let answer = host.input("Where do your projects live?", &guess.to_string_lossy())?;
    let answer = answer.trim();
    let projects_dir = match answer.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(answer),
    };

    let config = Config {
        projects_dirs: vec![projects_dir],
        username: username(host, &home),
        ..Config::default()
    };
    host.write_file(path, &render(&config))?;
    Ok(config)
}

/// `git config user.name`, slugified; the home directory's name when git has
/// no answer, which on any Unix is the login name.
fn username(host: &dyn Host, home: &Path) -> String {
    let configured = host
        .run(&["git", "config", "user.name"], None)
        .map(|out| slugify(out.trimmed()))
        .unwrap_or_default();
    if !configured.is_empty() {
        return configured;
    }
    slugify(&home.file_name().unwrap_or_default().to_string_lossy())
}

/// Lowercase, alphanumerics and hyphens, runs of hyphens collapsed.
fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn quote(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn quote_list(paths: &[PathBuf]) -> String {
    let items: Vec<String> = paths.iter().map(|p| quote(&p.to_string_lossy())).collect();
    format!("[{}]", items.join(", "))
}

fn render(config: &Config) -> String {
    let default_branch = match &config.default_branch {
        Some(branch) => format!("default_branch = {}", quote(branch)),
        None => "# default_branch = \"develop\"".to_string(),
    };
    format!(
        "# vibestation configuration. Delete any field to get its default back.

# Directories scanned for git repositories.
projects_dirs = {dirs}

# Repositories added by hand, or through the picker's add-manually action.
extra_projects = {extra}

# Prefix for generated session and branch names: username/TICKET-description.
username = {username}

# The branch new branches are cut from. Left out, it is detected per
# repository: origin/HEAD, then main, then master.
{default_branch}

# How deep below each projects directory to look for repositories.
scan_depth = {scan_depth}

# The pre-selected answer to \"fetch before branching?\".
fetch_before_branch = {fetch_before_branch}
",
        dirs = quote_list(&config.projects_dirs),
        extra = quote_list(&config.extra_projects),
        username = quote(&config.username),
        scan_depth = config.scan_depth,
        fetch_before_branch = config.fetch_before_branch,
    )
}
