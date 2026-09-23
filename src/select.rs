//! The list prompt: inquire's select, owned, so that ← and → can mean
//! something.
//!
//! inquire's select hands every key it does not bind to its filter line, from
//! a hardcoded match with no hook, so ← and → can only ever move a cursor
//! through the three characters you typed — the wall [`crate::line`] exists
//! for, met again. The filter line here is that module's line, so it edits the
//! way every other prompt does; the list, the paging and inquire's scorer are
//! this module's.
//!
//! A pure [`apply`] holds every binding and no I/O; the loop around it only
//! reads keys and redraws.

use crate::host::{Aborted, Pick};
use crate::line::{self, Line};
use anyhow::Result;
use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::queue;
use crossterm::style::{Color, Print, PrintStyledContent, StyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Reverse;
use std::io::{self, Write};

/// The list as the keys have left it.
struct List<'a> {
    options: &'a [String],
    filter: Line,
    /// What the filter lets through, best match first, as indexes into
    /// `options`.
    shown: Vec<usize>,
    /// Into `shown`.
    cursor: usize,
    matcher: SkimMatcherV2,
}

impl<'a> List<'a> {
    fn new(options: &'a [String]) -> Self {
        List {
            options,
            filter: Line::default(),
            shown: (0..options.len()).collect(),
            cursor: 0,
            // inquire's own scorer, so a filter finds what it found before.
            matcher: SkimMatcherV2::default().ignore_case(),
        }
    }

    fn highlighted(&self) -> Option<usize> {
        self.shown.get(self.cursor).copied()
    }

    /// Score every option against the filter. Ties keep list order — the list
    /// is ranked, and two rows a filter matches equally well should not trade
    /// places for it — which inquire's unstable sort did not promise.
    fn refilter(&mut self) {
        let typed: String = self.filter.chars.iter().collect();
        let mut scored: Vec<(usize, i64)> = self
            .options
            .iter()
            .enumerate()
            .filter_map(|(index, option)| Some((index, self.matcher.fuzzy_match(option, &typed)?)))
            .collect();
        scored.sort_by_key(|(_, score)| Reverse(*score));
        let shown: Vec<usize> = scored.into_iter().map(|(index, _)| index).collect();
        if shown != self.shown {
            self.shown = shown;
            self.cursor = 0;
        }
    }
}

/// What a key did to the prompt, rather than to the list.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    Continue,
    Chose(Pick),
    Cancel,
}

/// Every binding, and nothing else: no terminal, no I/O. `arrows` binds ←
/// and →; without it they move through the filter, as they always did.
fn apply(list: &mut List, key: KeyEvent, arrows: bool, page: usize) -> Step {
    if key.kind != KeyEventKind::Press {
        return Step::Continue;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let last = list.shown.len().saturating_sub(1);

    match (key.code, ctrl, alt) {
        // Up and down wrap, as inquire's did; a page and the ends do not.
        (KeyCode::Up, false, false) | (KeyCode::Char('p'), true, false) => {
            list.cursor = list.cursor.checked_sub(1).unwrap_or(last)
        }
        (KeyCode::Down, false, false) | (KeyCode::Char('n'), true, false) => {
            list.cursor = (list.cursor + 1) % list.shown.len().max(1)
        }
        (KeyCode::PageUp, ..) => list.cursor = list.cursor.saturating_sub(page),
        (KeyCode::PageDown, ..) => list.cursor = (list.cursor + page).min(last),
        (KeyCode::Home, ..) => list.cursor = 0,
        (KeyCode::End, ..) => list.cursor = last,

        // ← is about the whole list, so it needs no row under the cursor.
        (KeyCode::Left, false, false) if arrows => return Step::Chose(Pick::Left),
        (KeyCode::Right, false, false) if arrows => {
            if let Some(index) = list.highlighted() {
                return Step::Chose(Pick::Right(index));
            }
        }

        _ => match line::apply(&mut list.filter, key) {
            // A filter that matches nothing has nothing to choose.
            line::Step::Submit => {
                if let Some(index) = list.highlighted() {
                    return Step::Chose(Pick::Enter(index));
                }
            }
            line::Step::Cancel => return Step::Cancel,
            line::Step::Continue => list.refilter(),
        },
    }
    Step::Continue
}

/// How many rows the list may show: the terminal less the prompt line, the
/// help line and two rows of margin, so opening the list never scrolls the
/// prompt off its own screen. Floored at five — a very short terminal still
/// needs enough rows to be a list.
fn page_size(height: usize) -> usize {
    height.saturating_sub(4).max(5)
}

/// The first row on screen: the cursor held mid-page, as inquire held it,
/// until an end of the list is in view.
fn top(cursor: usize, len: usize, page: usize) -> usize {
    cursor
        .saturating_sub(page / 2)
        .min(len.saturating_sub(page))
}

/// Choose one of `options` in a terminal of `(width, height)`, with `help`
/// under the list, or [`Aborted`].
pub fn choose(
    message: &str,
    options: &[String],
    help: &str,
    arrows: bool,
    (width, height): (usize, usize),
) -> Result<Pick> {
    enable_raw_mode()?;
    let picked = read(message, options, help, arrows, width, page_size(height));
    let _ = disable_raw_mode();
    picked
}

fn read(
    message: &str,
    options: &[String],
    help: &str,
    arrows: bool,
    width: usize,
    page: usize,
) -> Result<Pick> {
    let mut list = List::new(options);
    let mut out = io::stdout().lock();

    loop {
        render(&mut out, message, &list, help, width, page)?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let step = match apply(&mut list, key, arrows, page) {
            Step::Continue => continue,
            step => step,
        };

        queue!(out, MoveToColumn(0), Clear(ClearType::FromCursorDown))?;
        let Step::Chose(pick) = step else {
            // An abandoned prompt keeps its `?`, as the text prompt's does.
            queue!(out, PrintStyledContent(paint("?", Color::Green)))?;
            queue!(out, Print(format!(" {message}\r\n")))?;
            out.flush()?;
            return Err(Aborted.into());
        };
        // A chosen row leaves the answered line inquire left. The arrows leave
        // nothing: what they open is drawn where the list was.
        if let Pick::Enter(index) = pick {
            queue!(
                out,
                PrintStyledContent(paint(">", Color::Green)),
                Print(format!(" {message} ")),
                PrintStyledContent(paint(options[index].trim(), Color::Cyan)),
                Print("\r\n"),
            )?;
        }
        out.flush()?;
        return Ok(pick);
    }
}

/// The prompt line, a page of the list and the help, with the cursor left in
/// the filter. Every line is cut to the terminal: one that wrapped would throw
/// off the move back up, and leave a copy of itself behind at every redraw.
fn render(
    out: &mut impl Write,
    message: &str,
    list: &List,
    help: &str,
    width: usize,
    page: usize,
) -> Result<()> {
    let filter: String = list.filter.chars.iter().collect();
    let top = top(list.cursor, list.shown.len(), page);
    let rows = &list.shown[top..(top + page).min(list.shown.len())];
    let more_below = top + rows.len() < list.shown.len();

    queue!(
        out,
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
        PrintStyledContent(paint("?", Color::Green)),
        Print(cut(
            &format!(" {message} {filter}"),
            width.saturating_sub(1)
        )),
    )?;
    for (at, &index) in rows.iter().enumerate() {
        let here = top + at == list.cursor;
        // `❯` is the cursor; `^` and `v` say the list goes on past the page.
        let prefix = match (here, at) {
            (true, _) => "❯",
            (_, 0) if top > 0 => "^",
            _ if at + 1 == rows.len() && more_below => "v",
            _ => " ",
        };
        let text = cut(&format!("{prefix} {}", list.options[index]), width);
        queue!(out, Print("\r\n"))?;
        match here {
            true => queue!(out, PrintStyledContent(paint(&text, Color::Cyan)))?,
            false => queue!(out, Print(text))?,
        }
    }
    let help = cut(&format!("[{help}]"), width);
    queue!(
        out,
        Print("\r\n"),
        PrintStyledContent(paint(&help, Color::Cyan))
    )?;

    let column = "? ".len() + message.chars().count() + 1 + list.filter.cursor;
    queue!(
        out,
        MoveUp(rows.len() as u16 + 1),
        MoveToColumn(column.min(width.saturating_sub(1)) as u16)
    )?;
    out.flush()?;
    Ok(())
}

fn cut(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

/// In the colours inquire used, honouring `NO_COLOR` as it did.
fn paint(text: &str, color: Color) -> StyledContent<String> {
    match std::env::var_os("NO_COLOR") {
        Some(_) => text.to_string().stylize(),
        None => text.to_string().with(color),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn typed(text: &str) -> Vec<KeyEvent> {
        text.chars().map(|c| plain(KeyCode::Char(c))).collect()
    }

    fn options() -> Vec<String> {
        ["ada/VBSN-4-picker", "notes-scratch", "api", "vibestation"]
            .map(String::from)
            .to_vec()
    }

    /// The step the last of `keys` ended on, with the arrows bound.
    fn press(keys: &[KeyEvent]) -> Step {
        let options = options();
        let mut list = List::new(&options);
        let mut step = Step::Continue;
        for key in keys {
            step = apply(&mut list, *key, true, 2);
        }
        step
    }

    #[test]
    fn enter_chooses_the_row_under_the_cursor_and_the_cursor_wraps() {
        use KeyCode::{Down, End, Enter, Home, PageDown, Up};

        assert_eq!(press(&[plain(Enter)]), Step::Chose(Pick::Enter(0)));
        assert_eq!(
            press(&[plain(Down), ctrl('n'), plain(Enter)]),
            Step::Chose(Pick::Enter(2))
        );
        assert_eq!(
            press(&[plain(Up), plain(Enter)]),
            Step::Chose(Pick::Enter(3)),
            "up from the first row wraps to the last"
        );
        assert_eq!(
            press(&[plain(End), plain(Down), plain(Enter)]),
            Step::Chose(Pick::Enter(0)),
            "and down from the last to the first"
        );
        assert_eq!(
            press(&[plain(End), ctrl('p'), plain(Home), plain(Enter)]),
            Step::Chose(Pick::Enter(0))
        );
        assert_eq!(
            press(&[
                plain(PageDown),
                plain(PageDown),
                plain(PageDown),
                plain(Enter)
            ]),
            Step::Chose(Pick::Enter(3)),
            "a page stops at the end rather than wrapping"
        );
    }

    #[test]
    fn typing_filters_and_enter_answers_with_the_original_index() {
        let mut keys = typed("vbsn");
        keys.push(plain(KeyCode::Enter));
        assert_eq!(press(&keys), Step::Chose(Pick::Enter(0)));

        let mut keys = typed("api");
        keys.push(plain(KeyCode::Enter));
        assert_eq!(press(&keys), Step::Chose(Pick::Enter(2)));

        let mut keys = typed("zzz");
        keys.push(plain(KeyCode::Enter));
        assert_eq!(
            press(&keys),
            Step::Continue,
            "nothing matched, nothing chosen"
        );
    }

    #[test]
    fn a_filter_that_matches_rows_equally_keeps_them_in_list_order() {
        let options = ["b-one", "a-one", "c-one"].map(String::from).to_vec();
        let mut list = List::new(&options);
        for key in typed("one") {
            apply(&mut list, key, true, 5);
        }
        assert_eq!(list.shown, [0, 1, 2]);
    }

    #[test]
    fn the_arrows_choose_the_list_and_the_row() {
        use KeyCode::{Down, Left, Right};

        assert_eq!(press(&[plain(Left)]), Step::Chose(Pick::Left));
        assert_eq!(
            press(&[plain(Down), plain(Right)]),
            Step::Chose(Pick::Right(1))
        );

        let mut keys = typed("zzz");
        keys.push(plain(Left));
        assert_eq!(press(&keys), Step::Chose(Pick::Left), "← needs no row");
        keys.pop();
        keys.push(plain(Right));
        assert_eq!(press(&keys), Step::Continue, "→ does");
    }

    #[test]
    fn unbound_arrows_edit_the_filter_as_they_did() {
        let options = options();
        let mut list = List::new(&options);
        for key in typed("ap") {
            apply(&mut list, key, false, 5);
        }

        assert_eq!(
            apply(&mut list, plain(KeyCode::Left), false, 5),
            Step::Continue
        );
        apply(&mut list, plain(KeyCode::Char('x')), false, 5);

        assert_eq!(list.filter.chars.iter().collect::<String>(), "axp");
    }

    #[test]
    fn the_filter_edits_with_the_line_keys_and_the_cancel_keys_abort() {
        let mut keys = typed("zzz");
        keys.push(ctrl('w'));
        keys.push(plain(KeyCode::Enter));
        assert_eq!(
            press(&keys),
            Step::Chose(Pick::Enter(0)),
            "C-w cleared the filter, so every row is back"
        );

        assert_eq!(press(&[plain(KeyCode::Esc)]), Step::Cancel);
        assert_eq!(press(&[ctrl('c')]), Step::Cancel);
    }

    #[test]
    fn the_page_fills_the_terminal_and_follows_the_cursor() {
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

        assert_eq!(top(0, 30, 10), 0);
        assert_eq!(top(4, 30, 10), 0, "the first half page does not scroll");
        assert_eq!(top(12, 30, 10), 7, "the cursor is held mid-page");
        assert_eq!(top(29, 30, 10), 20, "the last page is full, not half empty");
        assert_eq!(top(3, 4, 10), 0, "a list shorter than a page never scrolls");
    }
}
