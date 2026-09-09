//! Reading git state. Every call here is local — constitution §3 keeps the
//! network off the path to a session, and the one fetch that is allowed
//! arrives with ticket 09.

use crate::config::Config;
use crate::host::Host;
use anyhow::Result;
use std::path::Path;

/// The branch checked out in `path`, empty when it is not a repository.
pub fn branch(host: &dyn Host, path: &Path) -> Result<String> {
    let out = host.run(&["git", "rev-parse", "--abbrev-ref", "HEAD"], Some(path))?;
    Ok(match out.succeeded() {
        true => out.trimmed().to_string(),
        false => String::new(),
    })
}

/// Whether `branch` is the one new work would be cut from, and so the one that
/// means "this checkout has nothing started on it yet".
///
/// ponytail: the configured override and the two conventional names, which is
/// every repository that is not deliberately unusual. Ticket 09 replaces this
/// with the `origin/HEAD` → `main` → `master` detection.
pub fn is_default(config: &Config, branch: &str) -> bool {
    match &config.default_branch {
        Some(default) => branch == default,
        None => branch == "main" || branch == "master",
    }
}
