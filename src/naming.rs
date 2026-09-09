//! Turning what the developer says into a name they will recognise later:
//! `VBSN-1 initialize the project` becomes `ada/VBSN-1-initialize-the-project`.

use std::path::{Path, PathBuf};

/// The name for work that already has a branch: `username/<branch>`, without a
/// second prefix when the branch already carries one — branches this tool cuts
/// are `ada/VBSN-1-init` already.
pub fn from_branch(username: &str, branch: &str) -> String {
    let prefix = format!("{username}/");
    match username.is_empty() || branch.starts_with(&prefix) {
        true => sanitize(branch),
        false => sanitize(&format!("{prefix}{branch}")),
    }
}

/// The name suggested from a single line of prompt input. A leading
/// ticket-shaped token is kept as it was typed; everything after it — or the
/// whole line, when there is no ticket — becomes the slug.
pub fn suggest(username: &str, said: &str) -> String {
    let (ticket, description) = split_ticket(said.trim());
    let slug = truncate(&slugify(description), 50);
    let branch = match (ticket, slug.as_str()) {
        (Some(ticket), "") => ticket.to_string(),
        (Some(ticket), slug) => format!("{ticket}-{slug}"),
        (None, slug) => slug.to_string(),
    };
    match branch.is_empty() {
        true => sanitize(username),
        false => from_branch(username, &branch),
    }
}

/// Where a worktree for `branch` lands: a sibling of the main checkout, named
/// for it and the branch with the `username/` prefix stripped and any
/// remaining slashes hyphenated — `vibestation` and `ada/VBSN-1-init` give
/// `vibestation-VBSN-1-init`. Identifiable from a shell prompt, and an
/// ordinary directory to every other tool.
pub fn worktree_dir(main: &Path, username: &str, branch: &str) -> PathBuf {
    let stripped = match username.is_empty() {
        true => branch,
        false => branch
            .strip_prefix(&format!("{username}/"))
            .unwrap_or(branch),
    };
    let project = main.file_name().unwrap_or_default().to_string_lossy();
    main.with_file_name(format!("{project}-{}", stripped.replace('/', "-")))
}

/// tmux forbids `.` and `:` in session names. Slashes it permits, and they
/// carry the `username/` convention, so they stay.
pub fn sanitize(name: &str) -> String {
    name.replace(['.', ':'], "-")
}

/// Lowercase, alphanumerics and hyphens, runs of hyphens collapsed.
pub fn slugify(text: &str) -> String {
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

/// A ticket leads the line or there is no ticket: `VBSN-1 fix the parser`
/// splits, `fix VBSN-1 later` is all description.
fn split_ticket(said: &str) -> (Option<&str>, &str) {
    match said.split_once(char::is_whitespace) {
        Some((first, rest)) if is_ticket(first) => (Some(first), rest),
        _ if is_ticket(said) => (Some(said), ""),
        _ => (None, said),
    }
}

/// Ticket-shaped: uppercase letters, a hyphen, digits.
fn is_ticket(token: &str) -> bool {
    let Some((letters, digits)) = token.split_once('-') else {
        return false;
    };
    !letters.is_empty()
        && letters.chars().all(|c| c.is_ascii_uppercase())
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
}

/// Roughly `limit` characters, cut back to the last whole word rather than
/// leaving half of one.
fn truncate(slug: &str, limit: usize) -> String {
    if slug.chars().count() <= limit {
        return slug.to_string();
    }
    let cut: String = slug.chars().take(limit).collect();
    match cut.rsplit_once('-') {
        Some((whole, _)) => whole.to_string(),
        None => cut,
    }
}
