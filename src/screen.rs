//! The terminal as vibestation holds it while it runs: the alternate screen,
//! as vim and mc take it, given back exactly as it was found. Every frame is
//! drawn whole, at absolute rows, so a resize redraws the screen instead of
//! smearing a window drawn relative to wherever the cursor had got to.
//!
//! The picker fills the screen. Every other prompt is a box over it — the
//! picker as last drawn, dimmed — with its keys on the screen's last line.
//!
//! Raw mode is not held here but by each prompt while it reads keys, so that
//! between prompts — a fetch, an ssh passphrase — Ctrl-C and the tty work as
//! they always did.

use crate::line::paint;
use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::style::{Color, PrintStyledContent, StyledContent, Stylize};
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

/// A screen, line by line, each line in its colours.
pub type Window = Vec<Vec<StyledContent<String>>>;

/// One line of a box's body, before the box: its text in runs of colour.
pub type Body = Vec<(String, Option<Color>)>;

pub const DIM: Option<Color> = Some(Color::DarkGrey);

static TAKEN: AtomicBool = AtomicBool::new(false);

/// The picker as it was last drawn, as text: what a box is drawn over.
/// ponytail: a global, as there is one terminal; a field on the host if that
/// ever changes.
static BACKDROP: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn backdrop_lock() -> MutexGuard<'static, Vec<String>> {
    BACKDROP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Take the alternate screen, once; later calls do nothing.
pub fn take() -> Result<()> {
    if !TAKEN.swap(true, Ordering::SeqCst) {
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
    }
    Ok(())
}

/// Give the terminal back as it was found: before anything else is handed
/// it, before an error is printed, and before a panic says why. Does nothing
/// when it was never taken.
pub fn give_back() {
    if TAKEN.swap(false, Ordering::SeqCst) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

/// Draw `lines` over every row of a terminal `(width, height)`, and leave the
/// cursor at `(column, row)`, or hidden.
pub fn draw(
    lines: &Window,
    (width, height): (usize, usize),
    cursor: Option<(usize, usize)>,
) -> Result<()> {
    let mut out = io::stdout().lock();
    queue!(out, Hide)?;
    for row in 0..height {
        queue!(out, MoveTo(0, row as u16))?;
        let mut drawn = 0;
        for span in lines.get(row).into_iter().flatten() {
            drawn += span.content().chars().count();
            queue!(out, PrintStyledContent(span.clone()))?;
        }
        // A full line leaves the cursor on its last cell, which clearing to
        // the end of the line would rub out.
        if drawn < width {
            queue!(out, Clear(ClearType::UntilNewLine))?;
        }
    }
    if let Some((column, row)) = cursor {
        queue!(out, MoveTo(column as u16, row as u16), Show)?;
    }
    out.flush()?;
    Ok(())
}

/// Keep the picker as drawn, for the boxes opened over it.
pub fn set_backdrop(lines: &Window) {
    *backdrop_lock() = lines
        .iter()
        .map(|line| line.iter().map(|span| span.content().as_str()).collect())
        .collect();
}

/// The box's inner width: room for its title and its widest line, inside a
/// terminal `width` wide with a margin either side.
pub fn inner(title: &str, widest: usize, width: usize) -> usize {
    (title.chars().count() + 2)
        .max(widest)
        .min(width.saturating_sub(8))
        .max(1)
}

/// `body` in a box `inner` wide titled `title`, centred over the picker as
/// last drawn, dimmed, with `keys` on the last line. Also where the body's
/// first cell landed, for a prompt to put its cursor in.
pub fn dialog(
    title: &str,
    body: &[Body],
    inner: usize,
    keys: &str,
    size: (usize, usize),
) -> (Window, (usize, usize)) {
    over(&backdrop_lock(), Some((title, body, inner)), keys, size)
}

/// The picker as last drawn, dimmed and with nothing over it: what is left on
/// screen while the answer to a box is being acted on. The cursor waits on
/// the last line, where whatever git or ssh asks of the terminal will show.
pub fn rest(size: (usize, usize)) -> Result<()> {
    let (lines, _) = over(&backdrop_lock(), None, "", size);
    draw(&lines, size, Some((0, size.1.saturating_sub(1))))
}

/// [`dialog`], given the backdrop: pure, so the layout is testable.
fn over(
    backdrop: &[String],
    boxed: Option<(&str, &[Body], usize)>,
    keys: &str,
    (width, height): (usize, usize),
) -> (Window, (usize, usize)) {
    let rows = height.saturating_sub(1);
    let mut lines: Window = (0..rows)
        .map(|row| {
            let text = backdrop.get(row).map_or("", String::as_str);
            vec![dim(&cut(text, width))]
        })
        .collect();
    let mut origin = (0, 0);

    if let Some((title, body, inner)) = boxed {
        let wide = inner + 4;
        let tall = body.len().min(rows.saturating_sub(2)) + 2;
        let x = width.saturating_sub(wide) / 2;
        let y = rows.saturating_sub(tall) / 2;
        origin = (x + 2, y + 1);

        let title = cut(title, inner.saturating_sub(1));
        let rule = inner + 2 - title.chars().count().min(inner + 2);
        let mut boxed: Vec<Vec<StyledContent<String>>> = Vec::new();
        boxed.push(match title.is_empty() {
            true => vec![format!("┌{}┐", "─".repeat(inner + 2)).stylize()],
            false => vec![
                "┌─ ".to_string().stylize(),
                title.clone().bold(),
                format!(" {}┐", "─".repeat(rule.saturating_sub(3))).stylize(),
            ],
        });
        for line in body.iter().take(tall - 2) {
            let mut spans = vec!["│ ".to_string().stylize()];
            let mut room = inner;
            for (text, color) in line {
                let text = cut(text, room);
                room -= text.chars().count();
                spans.push(match color {
                    Some(color) => paint(&text, *color),
                    None => text.stylize(),
                });
            }
            spans.push(format!("{} │", " ".repeat(room)).stylize());
            boxed.push(spans);
        }
        boxed.push(vec![format!("└{}┘", "─".repeat(inner + 2)).stylize()]);

        for (at, spans) in boxed.into_iter().enumerate() {
            let Some(line) = lines.get_mut(y + at) else {
                break;
            };
            let under: Vec<char> = line[0].content().chars().collect();
            let left: String = under.iter().take(x).collect();
            let right: String = under.iter().skip(x + wide).collect();
            let mut spliced = vec![dim(&format!("{left:<x$}"))];
            spliced.extend(spans);
            spliced.push(dim(&right));
            *line = spliced;
        }
    }

    lines.push(vec![dim(&cut(keys, width))]);
    (lines, origin)
}

/// One line — how far a refresh has got — on the last row, over whatever the
/// screen holds; empty clears it.
pub fn status(line: &str, (width, height): (usize, usize)) {
    if take().is_err() {
        return;
    }
    let line = cut(line, width.saturating_sub(1));
    let mut out = io::stdout().lock();
    let _ = queue!(
        out,
        MoveTo(0, height.saturating_sub(1) as u16),
        Clear(ClearType::CurrentLine),
        PrintStyledContent(line.stylize())
    );
    let _ = out.flush();
}

/// The notes as the lines they take: wrapped rather than cut, since the
/// reason comes last — git's own error has several lines.
pub fn noted(notes: &[String], width: usize) -> Vec<String> {
    notes
        .iter()
        .flat_map(|note| note.lines())
        .flat_map(|line| wrap(line.trim_end(), width))
        .collect()
}

/// `text` in lines of at most `width`, broken at the last space that fits,
/// or mid-word when none does.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rest: Vec<char> = text.chars().collect();
    let mut lines = Vec::new();
    while rest.len() > width {
        let at = rest[..=width]
            .iter()
            .rposition(|c| *c == ' ')
            .filter(|at| *at > 0)
            .unwrap_or(width);
        lines.push(rest[..at].iter().collect::<String>().trim_end().to_string());
        rest = rest[at..]
            .iter()
            .skip_while(|c| **c == ' ')
            .copied()
            .collect();
    }
    lines.push(rest.into_iter().collect());
    lines
}

pub fn cut(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

fn dim(text: &str) -> StyledContent<String> {
    paint(text, Color::DarkGrey)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(window: &Window) -> Vec<String> {
        window
            .iter()
            .map(|line| line.iter().map(|span| span.content().as_str()).collect())
            .collect()
    }

    #[test]
    fn a_box_sits_centred_over_the_picker_with_its_keys_at_the_bottom() {
        let backdrop: Vec<String> = (0..6)
            .map(|row| format!("{row}{}", "-".repeat(19)))
            .collect();
        let body = [
            vec![("❯ 1. Yes".to_string(), Some(Color::Cyan))],
            vec![("  2. No".to_string(), None)],
        ];

        let (window, origin) = over(&backdrop, Some(("Remove?", &body, 9)), "Enter", (20, 7));

        assert_eq!(
            text(&window),
            [
                "0-------------------",
                "1--┌─ Remove? ─┐----",
                "2--│ ❯ 1. Yes  │----",
                "3--│   2. No   │----",
                "4--└───────────┘----",
                "5-------------------",
                "Enter",
            ]
        );
        assert_eq!(origin, (5, 2), "the body's first cell");
        assert_eq!(window.len(), 7, "every row of the terminal");
    }

    #[test]
    fn the_picker_shows_either_side_of_a_narrow_box() {
        let backdrop = ["abcdefghijklmnopqrst".to_string()];
        let body = [vec![("x".to_string(), None)]];
        let (window, _) = over(&backdrop, Some(("", &body, 2)), "", (20, 4));
        assert_eq!(text(&window)[0], "abcdefg┌────┐nopqrst");
        assert_eq!(text(&window)[1], "       │ x  │");
    }

    #[test]
    fn a_long_line_wraps_at_a_space_or_mid_word_when_there_is_none() {
        assert_eq!(
            wrap("Notes: fixing the arrows", 12),
            ["Notes:", "fixing the", "arrows"]
        );
        assert_eq!(wrap("abcdefgh", 3), ["abc", "def", "gh"]);
        assert_eq!(wrap("short", 12), ["short"]);
        assert_eq!(wrap("", 12), [""]);
    }

    #[test]
    fn a_box_is_as_wide_as_its_title_or_widest_line_and_fits_the_terminal() {
        assert_eq!(inner("Remove?", 9, 80), 9);
        assert_eq!(inner("A much longer question here", 9, 80), 29);
        assert_eq!(inner("t", 200, 80), 72, "eight columns of margin");
    }
}
