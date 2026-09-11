//! The one picker: live sessions, a separator, then projects ranked by
//! frecency with their worktrees as indented children, and last the refresh
//! and add-manually actions.
//!
//! Every row is laid out in the same six columns — what it is, what it is
//! called, where it is, its branch, what is running, how long since it was
//! last attached — each padded to its width across the whole list, so
//! sessions, projects and worktrees read as one grid rather than three. The
//! grid is then fitted to the terminal: inquire wraps a row that does not fit,
//! and one wrapped row ruins the list.

use crate::scan::Project;
use crate::tmux::Session;
use std::path::Path;

/// What a row stands for, by index into the lists it was built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Session(usize),
    Project(usize),
    /// A project and one of its worktrees.
    Worktree(usize, usize),
    /// The line between the sessions and everything else. Selectable, because
    /// no prompt library can make a row inert; choosing it reopens the picker,
    /// since decoration should cost nothing.
    Separator,
    /// Rescan the configured roots and rewrite the cache.
    Refresh,
    /// Take a repository by path, for one that lives outside those roots.
    AddManually,
}

/// The columns, in order.
const GLYPH: usize = 0;
const NAME: usize = 1;
const DIR: usize = 2;
const BRANCH: usize = 3;
const COMMAND: usize = 4;
const AGE: usize = 5;
const COLUMNS: usize = 6;

/// One row's cells, one per column.
type Cells = [String; COLUMNS];

/// Between two columns.
const GUTTER: usize = 2;
/// What inquire writes before every option: a one-cell prefix and a space.
const PREFIX: usize = 2;

/// Below these a column has stopped saying anything, so it is given up whole
/// rather than shaved further.
const MIN_NAME: usize = 6;
const MIN_DIR: usize = 8;
const MIN_BRANCH: usize = 6;

/// A path gives up its front freely down to this. Below it the path is being
/// destroyed, and the command — the least load-bearing column there is — goes
/// before that happens.
const SOFT_DIR: usize = 18;

/// The rows, in picker order. A project or worktree that already has a live
/// session is left out: its session row above is the same work.
pub fn rows(
    sessions: &[Session],
    projects: &[Project],
    home: &Path,
    username: &str,
    width: usize,
) -> Vec<(Row, String)> {
    let live: Vec<&Path> = sessions.iter().map(|s| s.path.as_path()).collect();

    let mut table: Vec<(Row, Option<Cells>)> = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let cells = cells(&[
                // `▶` is the session you are in, `●` a client somewhere else,
                // `○` a session running with nobody in it at all.
                match (session.current, session.attached) {
                    (true, _) => "▶",
                    (_, true) => "●",
                    _ => "○",
                },
                &session.name,
                &abbreviate(&session.path.to_string_lossy(), home),
                strip(&session.branch, username),
                &session.command,
                &session.age,
            ]);
            (Row::Session(index), Some(cells))
        })
        .collect();

    let mut below = Vec::new();
    for (index, project) in projects.iter().enumerate() {
        if !live.contains(&project.path.as_path()) {
            let path = abbreviate(&project.path.to_string_lossy(), home);
            let cells = cells(&["", &project.name, &path, "", "", ""]);
            below.push((Row::Project(index), Some(cells)));
        }
        for (child, worktree) in project.worktrees.iter().enumerate() {
            if live.contains(&worktree.path.as_path()) {
                continue;
            }
            // The corner does the nesting the indent used to, which keeps the
            // branch in the same column as every session's branch.
            let cells = cells(&[
                "└",
                &base(&worktree.path),
                "",
                strip(&worktree.branch, username),
                "",
                "",
            ]);
            below.push((Row::Worktree(index, child), Some(cells)));
        }
    }

    if !table.is_empty() {
        table.push((Row::Separator, None));
    }
    table.extend(below);
    // The escape hatches, last: nobody reaches for them until the list is
    // wrong, and they are why the picker is never empty.
    table.push((Row::Refresh, None));
    table.push((Row::AddManually, None));

    render(table, width.saturating_sub(PREFIX))
}

/// The prompt line, which is the one line inquire never scrolls away — so it
/// is where what the list is made of belongs. Worktrees credit their project,
/// as they do everywhere else, and a project showing as its own live session
/// is counted as the session it is.
pub fn title(rows: &[(Row, String)]) -> String {
    let count = |kind: fn(&Row) -> bool| rows.iter().filter(|(row, _)| kind(row)).count();
    let running = count(|row| matches!(row, Row::Session(_)));
    let projects = count(|row| matches!(row, Row::Project(_)));
    let listed = match projects {
        1 => "1 project".to_string(),
        _ => format!("{projects} projects"),
    };
    match running {
        0 => format!("Open  {listed}"),
        _ => format!("Open  {running} running · {listed}"),
    }
}

fn cells(fields: &[&str; COLUMNS]) -> Cells {
    (*fields).map(str::to_string)
}

/// Cap, fit, then draw. The separator and the two actions are not grid rows —
/// they have nothing to line up with — so they are drawn from the row itself,
/// starting where the name column starts.
fn render(mut table: Vec<(Row, Option<Cells>)>, budget: usize) -> Vec<(Row, String)> {
    cap(&mut table, budget);
    let widths = fit(&table, budget);
    let rule = line(&widths).min(budget);

    table
        .into_iter()
        .map(|(row, cells)| {
            let text = match (&cells, row) {
                (Some(cells), _) => join(cells, &widths),
                (None, Row::Separator) => "─".repeat(rule),
                (None, Row::Refresh) => "↻  refresh the project list".to_string(),
                (None, _) => "✚  add a project by path".to_string(),
            };
            // Whatever happened above, no row wraps.
            (row, truncate(&text, budget))
        })
        .collect()
}

/// No column may claim more than its share of the budget, so one long worktree
/// name cannot widen the name column for every other row.
fn cap(table: &mut [(Row, Option<Cells>)], budget: usize) {
    let caps = [1, budget / 3, budget / 3, budget / 4, 12, 4];
    for (_, cells) in table
        .iter_mut()
        .filter_map(|(row, cells)| Some((row, cells.as_mut()?)))
    {
        cells[DIR] = elide(&cells[DIR], caps[DIR]);
        for column in [GLYPH, NAME, BRANCH, COMMAND, AGE] {
            cells[column] = truncate(&cells[column], caps[column]);
        }
    }
}

/// The width of each column, shrunk until a row fits. What goes, goes in the
/// order a developer would have deleted it themselves: the path loses its
/// front, then the command, then the age, then the branch and the name are
/// shortened. A priority list, not a layout engine.
fn fit(table: &[(Row, Option<Cells>)], budget: usize) -> [usize; COLUMNS] {
    let mut widths = [0; COLUMNS];
    for cells in table.iter().filter_map(|(_, cells)| cells.as_ref()) {
        for column in 0..COLUMNS {
            widths[column] = widths[column].max(width(&cells[column]));
        }
    }

    let mut over = line(&widths).saturating_sub(budget);
    over = shave(&mut widths, DIR, SOFT_DIR, over);
    // Half a command name is noise, so these two go whole or not at all.
    over = give_up(&mut widths, COMMAND, over);
    over = give_up(&mut widths, AGE, over);
    over = shave(&mut widths, DIR, MIN_DIR, over);
    over = shave(&mut widths, BRANCH, MIN_BRANCH, over);
    shave(&mut widths, NAME, MIN_NAME, over);
    widths
}

/// Narrow one column towards its floor, returning what is still over.
fn shave(widths: &mut [usize; COLUMNS], column: usize, floor: usize, over: usize) -> usize {
    let give = over.min(widths[column].saturating_sub(floor));
    widths[column] -= give;
    over - give
}

/// Drop one column entirely, which also frees its gutter.
fn give_up(widths: &mut [usize; COLUMNS], column: usize, over: usize) -> usize {
    if over == 0 || widths[column] == 0 {
        return over;
    }
    let freed = widths[column] + GUTTER;
    widths[column] = 0;
    over.saturating_sub(freed)
}

/// The width of a row at these column widths. A column of width zero is not
/// there at all, gutter included.
fn line(widths: &[usize; COLUMNS]) -> usize {
    let used: Vec<usize> = widths.iter().copied().filter(|w| *w > 0).collect();
    used.iter().sum::<usize>() + GUTTER * used.len().saturating_sub(1)
}

/// One row at the settled widths, padded into a grid and stripped of the
/// padding that runs off the end of it.
fn join(cells: &Cells, widths: &[usize; COLUMNS]) -> String {
    let mut row = String::new();
    for column in 0..COLUMNS {
        let cell = widths[column];
        if cell == 0 {
            continue;
        }
        if !row.is_empty() {
            row.push_str(&" ".repeat(GUTTER));
        }
        let text = match column {
            DIR => elide(&cells[column], cell),
            _ => truncate(&cells[column], cell),
        };
        row.push_str(&format!("{text:<cell$}"));
    }
    row.trim_end().to_string()
}

fn width(text: &str) -> usize {
    text.chars().count()
}

/// `refresh the project list` in twelve cells is `refresh the…`.
fn truncate(text: &str, max: usize) -> String {
    if width(text) <= max {
        return text.to_string();
    }
    match max {
        0 => String::new(),
        _ => text.chars().take(max - 1).chain(['…']).collect(),
    }
}

/// `~/projects/sports/android-monorepo-3` in twenty-four cells is
/// `~/…/android-monorepo-3`: the end of a path is what identifies it, so what
/// goes is the front, a whole segment at a time.
fn elide(path: &str, max: usize) -> String {
    if width(path) <= max {
        return path.to_string();
    }
    let (head, rest) = match path.strip_prefix("~/") {
        Some(rest) => ("~/", rest),
        None => ("/", path.trim_start_matches('/')),
    };
    let segments: Vec<&str> = rest.split('/').collect();
    for keep in (1..segments.len()).rev() {
        let candidate = format!("{head}…/{}", segments[segments.len() - keep..].join("/"));
        if width(&candidate) <= max {
            return candidate;
        }
    }
    // Even the last segment alone overruns: there is nothing left to give up.
    truncate(segments.last().unwrap_or(&rest), max)
}

fn base(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// `/home/dev/code/api` as `~/code/api`, so the useful end of a long path is
/// what the eye lands on.
fn abbreviate(path: &str, home: &Path) -> String {
    match path.strip_prefix(&*home.to_string_lossy()) {
        Some("") => "~".to_string(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_string(),
    }
}

/// `ada/VBSN-1-init` as `VBSN-1-init`. Your own prefix on every branch of
/// every row is your own name, repeated as many times as you have rows.
fn strip<'a>(branch: &'a str, username: &str) -> &'a str {
    if username.is_empty() {
        return branch;
    }
    branch
        .strip_prefix(&format!("{username}/"))
        .unwrap_or(branch)
}
