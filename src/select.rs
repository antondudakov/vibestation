//! Every list prompt. The picker fills the screen: a rule, the question, the
//! rows with `❯` on the one under the cursor and a panel beside them
//! previewing it, a rule, and the keys. Everything else — a row's menu, the
//! branch question, yes or no — is a handful of options in a box over it,
//! numbered, so a digit chooses.
//!
//! The picker filters as you type, on [`crate::line`]'s line and skim's
//! scorer, since a ticket number is how you reach a worktree.
//!
//! A pure [`apply`] holds every binding, and a pure [`frame`] and [`menu`]
//! every line drawn; the loop around them only reads keys and draws.

use crate::host::{Aborted, Pick};
use crate::line::{self, paint, Line};
use crate::screen::{self, cut, wrap, Body, Window, DIM};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, StyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Reverse;
use std::sync::{Mutex, MutexGuard};

/// A list, as the host asks for one.
#[derive(Default)]
pub struct Ask<'a> {
    pub message: &'a str,
    pub options: &'a [String],
    /// Beside the list, a panel for the option under the cursor. Empty for no
    /// panel. An option whose own is empty is a heading — `── projects ──` —
    /// drawn dim, passed over by the cursor and hidden by a filter: it has
    /// nothing to show because there is nothing there to choose.
    pub previews: &'a [Vec<String>],
    /// The keys this list binds beyond those every list does, for the line
    /// under it.
    pub keys: &'a str,
    /// Typing filters and Tab chooses. Otherwise the options are numbered.
    pub filter: bool,
    /// ← and → choose.
    pub arrows: bool,
    /// The option the cursor starts on.
    pub cursor: usize,
    /// What the last action had to say — why a worktree was not removed —
    /// under the question, where it cannot scroll away.
    pub notes: &'a [String],
}

/// The list as the keys have left it.
struct List<'a> {
    ask: &'a Ask<'a>,
    filter: Line,
    /// What the filter lets through, best match first, as indexes into
    /// `options`.
    shown: Vec<usize>,
    /// Into `shown`.
    cursor: usize,
    matcher: SkimMatcherV2,
}

impl<'a> List<'a> {
    fn new(ask: &'a Ask<'a>) -> Self {
        let mut list = List {
            ask,
            filter: Line::default(),
            shown: (0..ask.options.len()).collect(),
            cursor: ask.cursor.min(ask.options.len().saturating_sub(1)),
            // inquire's scorer, so a filter finds what it always found.
            matcher: SkimMatcherV2::default().ignore_case(),
        };
        list.settle(true);
        if ask.filter {
            if let Some((typed, at, row)) = resume().take() {
                list.filter = Line {
                    chars: typed,
                    cursor: at,
                };
                list.refilter();
                if let Some(at) = list.shown.iter().position(|index| Some(*index) == row) {
                    list.cursor = at;
                }
            }
        }
        list
    }

    fn heading(&self, index: usize) -> bool {
        self.ask.previews.get(index).is_some_and(Vec::is_empty)
    }

    /// The option under the cursor, which is never a heading.
    fn highlighted(&self) -> Option<usize> {
        self.shown
            .get(self.cursor)
            .copied()
            .filter(|index| !self.heading(*index))
    }

    /// One row up or down, over any heading, wrapping at the ends.
    fn step(&mut self, forward: bool) {
        let len = self.shown.len();
        for _ in 0..len {
            self.cursor = match forward {
                true => (self.cursor + 1) % len,
                false => self.cursor.checked_sub(1).unwrap_or(len - 1),
            };
            if !self.heading(self.shown[self.cursor]) {
                return;
            }
        }
    }

    /// Off a heading the cursor jumped onto: the nearest row the way it was
    /// going, else the nearest the other way.
    fn settle(&mut self, forward: bool) {
        let len = self.shown.len();
        if len == 0 {
            return;
        }
        let choosable = |at: &usize| !self.heading(self.shown[*at]);
        let after = || (self.cursor..len).find(choosable);
        let before = || (0..=self.cursor).rev().find(choosable);
        let found = match forward {
            true => after().or_else(before),
            false => before().or_else(after),
        };
        if let Some(at) = found {
            self.cursor = at;
        }
    }

    /// Score every option against the filter. Ties keep list order — the list
    /// is ranked, and two rows a filter matches equally well should not trade
    /// places for it.
    fn refilter(&mut self) {
        let typed: String = self.filter.chars.iter().collect();
        let mut scored: Vec<(usize, i64)> = self
            .ask
            .options
            .iter()
            .enumerate()
            // A heading heads nothing once the rows under it are filtered.
            .filter(|(index, _)| typed.is_empty() || !self.heading(*index))
            .filter_map(|(index, option)| Some((index, self.matcher.fuzzy_match(option, &typed)?)))
            .collect();
        scored.sort_by_key(|(_, score)| Reverse(*score));
        let shown: Vec<usize> = scored.into_iter().map(|(index, _)| index).collect();
        if shown != self.shown {
            self.shown = shown;
            self.cursor = 0;
            self.settle(true);
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

/// Every binding, and nothing else: no terminal, no I/O.
fn apply(list: &mut List, key: KeyEvent, page: usize) -> Step {
    if key.kind != KeyEventKind::Press {
        return Step::Continue;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let last = list.shown.len().saturating_sub(1);
    let chose = |pick: fn(usize) -> Pick, list: &List| match list.highlighted() {
        Some(index) => Step::Chose(pick(index)),
        None => Step::Continue,
    };

    match (key.code, ctrl, alt) {
        // Up and down wrap; a page and the ends do not.
        (KeyCode::Up, false, false) | (KeyCode::Char('p'), true, false) => list.step(false),
        (KeyCode::Down, false, false) | (KeyCode::Char('n'), true, false) => list.step(true),
        (KeyCode::PageUp, ..) => {
            list.cursor = list.cursor.saturating_sub(page);
            list.settle(false)
        }
        (KeyCode::PageDown, ..) => {
            list.cursor = (list.cursor + page).min(last);
            list.settle(true)
        }
        (KeyCode::Home, ..) => {
            list.cursor = 0;
            list.settle(true)
        }
        (KeyCode::End, ..) => {
            list.cursor = last;
            list.settle(false)
        }

        // ← is about the whole list, so it needs no row under the cursor.
        (KeyCode::Left, false, false) if list.ask.arrows => return Step::Chose(Pick::Left),
        (KeyCode::Right, false, false) if list.ask.arrows => return chose(Pick::Right, list),
        (KeyCode::Tab, ..) if list.ask.filter => return chose(Pick::Tab, list),

        _ if list.ask.filter => match line::apply(&mut list.filter, key) {
            // A filter that matches nothing has nothing to choose.
            line::Step::Submit => return chose(Pick::Enter, list),
            line::Step::Cancel => return Step::Cancel,
            line::Step::Continue => list.refilter(),
        },

        (KeyCode::Enter, ..) => return chose(Pick::Enter, list),
        (KeyCode::Esc, ..) | (KeyCode::Char('c' | 'g'), true, false) => return Step::Cancel,
        (KeyCode::Char(digit @ '1'..='9'), false, false) => {
            let at = digit as usize - '1' as usize;
            if let Some(&index) = list.shown.get(at) {
                return Step::Chose(Pick::Enter(index));
            }
        }
        // The one option a letter starts, under the cursor for Enter to take:
        // `y` and `n` at a yes or no, typed as they always were.
        (KeyCode::Char(letter), false, false) => {
            let starts = |at: &usize| {
                let option = &list.ask.options[list.shown[*at]];
                option
                    .chars()
                    .next()
                    .is_some_and(|first| first.eq_ignore_ascii_case(&letter))
            };
            let matching: Vec<usize> = (0..list.shown.len()).filter(starts).collect();
            if let [only] = matching[..] {
                list.cursor = only;
            }
        }
        _ => {}
    }
    Step::Continue
}

/// Where the picker was when a resize sent its rows back to be laid out at
/// the new width — what was typed, where in it, and the row under the cursor
/// — for the picker that reopens to start from. Taken by that one alone.
static RESUME: Mutex<Option<Place>> = Mutex::new(None);

/// What was typed, the cursor in it, and the option under the list's cursor.
type Place = (Vec<char>, usize, Option<usize>);

fn resume() -> MutexGuard<'static, Option<Place>> {
    RESUME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// How many rows the picker shows: the screen less its five lines — two
/// rules, the question, the gap under it and the keys — and the notes.
fn page_size(height: usize, notes: usize) -> usize {
    height.saturating_sub(5 + notes).max(1)
}

/// The first row on screen: the cursor held mid-page until an end of the list
/// is in view.
fn top(cursor: usize, len: usize, page: usize) -> usize {
    cursor
        .saturating_sub(page / 2)
        .min(len.saturating_sub(page))
}

/// The gap before the panel, and its `│ ` and ` │`.
const PANEL_CHROME: usize = 6;

/// A terminal `width` wide split into the list's width and the width of the
/// panel's text. A third of the terminal, up to 64 columns.
///
/// ponytail: below 100 columns the grid needs every one and there is no panel —
/// and with it no note on screen. Stack the panel under the list if narrow
/// terminals come to matter.
pub fn split(width: usize) -> (usize, usize) {
    if width < 100 {
        return (width, 0);
    }
    let panel = (width / 3).min(64);
    (width - panel, panel - PANEL_CHROME)
}

/// Choose one of the options on a screen `(width, height)`, or [`Aborted`].
pub fn choose(ask: &Ask, size: (usize, usize)) -> Result<Pick> {
    screen::take()?;
    enable_raw_mode()?;
    let picked = read(ask, size);
    let _ = disable_raw_mode();
    picked
}

fn read(ask: &Ask, mut size: (usize, usize)) -> Result<Pick> {
    let mut list = List::new(ask);
    loop {
        let page = match ask.filter {
            true => {
                let notes = screen::noted(ask.notes, size.0.saturating_sub(2)).len();
                let page = page_size(size.1, notes);
                let (lines, column) = frame(&list, size.0, page);
                screen::set_backdrop(&lines);
                screen::draw(&lines, size, column.map(|column| (column, 1)))?;
                page
            }
            false => {
                let (lines, page) = menu(&list, size);
                screen::draw(&lines, size, None)?;
                page
            }
        };

        let key = match event::read()? {
            Event::Key(key) => key,
            Event::Resize(width, height) => {
                size = (width as usize, height as usize);
                // The picker's rows were laid out for the old width, so they
                // go back to be laid out again, and it reopens where it was.
                // A box's options fit any width as they are.
                if ask.filter {
                    let typed = list.filter.chars.clone();
                    *resume() = Some((typed, list.filter.cursor, list.highlighted()));
                    return Ok(Pick::Resize);
                }
                continue;
            }
            _ => continue,
        };
        match apply(&mut list, key, page) {
            Step::Continue => {}
            Step::Chose(pick) => {
                // What the answer sets going happens under the picker, not
                // under a box that still looks unanswered.
                if !ask.filter {
                    screen::rest(size)?;
                }
                return Ok(pick);
            }
            Step::Cancel => return Err(Aborted.into()),
        }
    }
}

/// One line being written, cut at the screen's edge: one that wrapped would
/// push every line under it down a row.
struct Spans {
    spans: Vec<StyledContent<String>>,
    room: usize,
}

impl Spans {
    fn new(width: usize) -> Self {
        Spans {
            spans: Vec::new(),
            room: width,
        }
    }

    fn push(&mut self, text: &str, color: Option<Color>) -> &mut Self {
        let text = cut(text, self.room);
        self.room -= text.chars().count();
        self.spans.push(match color {
            Some(color) => paint(&text, color),
            None => text.stylize(),
        });
        self
    }
}

/// The picker, every line of the screen, and the column the cursor sits at
/// on the question line. Pure, so the layout is testable.
fn frame(list: &List, width: usize, page: usize) -> (Window, Option<usize>) {
    let ask = list.ask;
    let rule = || {
        let mut line = Spans::new(width);
        line.push(&"─".repeat(width), DIM);
        line.spans
    };
    let mut lines = vec![rule()];

    let mut question = Spans::new(width.saturating_sub(1));
    question.push("│ ", DIM);
    question.spans.push(cut(ask.message, question.room).bold());
    question.room = question.room.saturating_sub(ask.message.chars().count());
    let typed: String = list.filter.chars.iter().collect();
    question.push("  ", None);
    match typed.is_empty() {
        true => question.push("type to filter", DIM),
        false => question.push(&typed, None),
    };
    let column = "│ ".chars().count() + ask.message.chars().count() + 2 + list.filter.cursor;
    lines.push(question.spans);
    for note in screen::noted(ask.notes, width.saturating_sub(2)) {
        let mut line = Spans::new(width);
        line.push("│ ", DIM).push(&note, Some(Color::Yellow));
        lines.push(line.spans);
    }
    lines.push(Vec::new());

    let top = top(list.cursor, list.shown.len(), page);
    let rows = &list.shown[top..(top + page).min(list.shown.len())];
    let more_below = top + rows.len() < list.shown.len();
    let (left, inner) = match ask.previews.is_empty() {
        true => (width, 0),
        false => split(width),
    };
    let preview = list
        .highlighted()
        .and_then(|index| ask.previews.get(index))
        .filter(|preview| inner > 0 && !preview.is_empty());
    let panel = preview.map_or(Vec::new(), |preview| boxed(preview, inner, page));

    // Every row of the page, so the keys sit on the screen's last line.
    for at in 0..page {
        let mut line = Spans::new(width);
        let text = match rows.get(at) {
            // `❯` is the cursor; `^` and `v` say the list goes on past the
            // page.
            Some(&index) => {
                let marker = match (top + at == list.cursor, at) {
                    (true, _) => "❯",
                    (_, 0) if top > 0 => "^",
                    _ if at + 1 == rows.len() && more_below => "v",
                    _ => " ",
                };
                format!("{marker} {}", ask.options[index])
            }
            None => String::new(),
        };
        let color = match rows.get(at) {
            Some(_) if top + at == list.cursor => Some(Color::Cyan),
            Some(&index) if list.heading(index) => DIM,
            _ => None,
        };
        match panel.get(at) {
            Some(boxed) => {
                line.push(&format!("{:<left$}", cut(&text, left)), color);
                for (text, color) in boxed {
                    line.push(text, *color);
                }
            }
            None => {
                line.push(&text, color);
            }
        }
        lines.push(line.spans);
    }

    lines.push(rule());
    let mut keys = Spans::new(width);
    keys.push(&footer(ask), DIM);
    lines.push(keys.spans);
    (lines, Some(column.min(width.saturating_sub(1))))
}

/// A list that does not filter, as a box over the picker: the notes, then the
/// options, numbered and paged to the screen. Also the page, for the keys
/// that move by one. Pure, so the layout is testable.
fn menu(list: &List, (width, height): (usize, usize)) -> (Window, usize) {
    let ask = list.ask;
    let widest = ask
        .options
        .iter()
        .map(|option| option.chars().count() + "❯ 1. ".chars().count())
        .chain(ask.notes.iter().map(|note| note.chars().count()))
        .max()
        .unwrap_or(0);
    let inner = screen::inner(ask.message, widest, width);

    let mut body: Vec<Body> = screen::noted(ask.notes, inner)
        .into_iter()
        .map(|note| vec![(note, Some(Color::Yellow))])
        .collect();
    if !body.is_empty() {
        body.push(Vec::new());
    }
    // The box's two edges, the keys, and a row of margin.
    let page = height.saturating_sub(4 + body.len()).max(1);
    let top = top(list.cursor, list.shown.len(), page);
    let shown = list.shown.len().min(top + page);
    for at in top..shown {
        let here = at == list.cursor;
        let marker = match here {
            true => "❯",
            false if at == top && top > 0 => "^",
            false if at + 1 == shown && shown < list.shown.len() => "v",
            false => " ",
        };
        let index = list.shown[at];
        body.push(vec![(
            format!("{marker} {}. {}", index + 1, ask.options[index]),
            here.then_some(Color::Cyan),
        )]);
    }
    let (lines, _) = screen::dialog(ask.message, &body, inner, &footer(ask), (width, height));
    (lines, page)
}

/// The preview in a box `inner` wide, cut to the page: its long lines wrapped,
/// the frame dim and the text plain.
fn boxed(preview: &[String], inner: usize, page: usize) -> Vec<Vec<(String, Option<Color>)>> {
    let edge = "─".repeat(inner + 2);
    let mut lines = vec![vec![(format!("  ┌{edge}┐"), DIM)]];
    let text = preview.iter().flat_map(|line| wrap(line, inner));
    for text in text.take(page.saturating_sub(2)) {
        lines.push(vec![
            ("  │ ".to_string(), DIM),
            (format!("{text:<inner$}"), None),
            (" │".to_string(), DIM),
        ]);
    }
    lines.push(vec![(format!("  └{edge}┘"), DIM)]);
    lines
}

/// The line of keys under the window, Claude Code's words for them.
fn footer(ask: &Ask) -> String {
    let choose = match (ask.filter, ask.options.len().min(9)) {
        (true, _) => "type to filter".to_string(),
        (false, 1) => "1 to choose".to_string(),
        (false, n) => format!("1-{n} to choose"),
    };
    [
        "Enter to select",
        "↑/↓ to navigate",
        &choose,
        ask.keys,
        "Esc to cancel",
    ]
    .into_iter()
    .filter(|keys| !keys.is_empty())
    .collect::<Vec<_>>()
    .join(" · ")
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

    /// The picker: filtered, with the arrows and Tab bound.
    fn picker(options: &[String]) -> Ask<'_> {
        Ask {
            options,
            filter: true,
            arrows: true,
            ..Ask::default()
        }
    }

    /// The step the last of `keys` ended on.
    fn press_at(ask: &Ask, keys: &[KeyEvent]) -> Step {
        let mut list = List::new(ask);
        let mut step = Step::Continue;
        for key in keys {
            step = apply(&mut list, *key, 2);
        }
        step
    }

    fn press(keys: &[KeyEvent]) -> Step {
        press_at(&picker(&options()), keys)
    }

    /// The window as text, styles dropped.
    fn text(ask: &Ask, keys: &[KeyEvent], width: usize) -> Vec<String> {
        let mut list = List::new(ask);
        for key in keys {
            apply(&mut list, *key, 10);
        }
        frame(&list, width, 10)
            .0
            .iter()
            .map(|line| line.iter().map(|span| span.content().as_str()).collect())
            .collect()
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
        let ask = picker(&options);
        let mut list = List::new(&ask);
        for key in typed("one") {
            apply(&mut list, key, 5);
        }
        assert_eq!(list.shown, [0, 1, 2]);
    }

    #[test]
    fn the_arrows_and_tab_choose_the_list_and_the_row() {
        use KeyCode::{Down, Left, Right, Tab};

        assert_eq!(press(&[plain(Left)]), Step::Chose(Pick::Left));
        assert_eq!(
            press(&[plain(Down), plain(Right)]),
            Step::Chose(Pick::Right(1))
        );
        assert_eq!(press(&[plain(Down), plain(Tab)]), Step::Chose(Pick::Tab(1)));

        let mut keys = typed("zzz");
        keys.push(plain(Left));
        assert_eq!(press(&keys), Step::Chose(Pick::Left), "← needs no row");
        keys.pop();
        keys.push(plain(Right));
        assert_eq!(press(&keys), Step::Continue, "→ does");
        keys.pop();
        keys.push(plain(Tab));
        assert_eq!(press(&keys), Step::Continue, "and so does Tab");
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
    fn a_numbered_list_chooses_by_digit_and_a_letter_moves_to_its_option() {
        use KeyCode::{Char, Enter, Esc, Left, Tab};
        let options = ["Join", "Kill", "Rename"].map(String::from).to_vec();
        let menu = Ask {
            options: &options,
            ..Ask::default()
        };

        assert_eq!(
            press_at(&menu, &[plain(Char('3'))]),
            Step::Chose(Pick::Enter(2))
        );
        assert_eq!(
            press_at(&menu, &[plain(Char('4'))]),
            Step::Continue,
            "no fourth option"
        );
        assert_eq!(
            press_at(&menu, &[plain(Char('k'))]),
            Step::Continue,
            "a letter only moves the cursor…"
        );
        assert_eq!(
            press_at(&menu, &[plain(Char('k')), plain(Enter)]),
            Step::Chose(Pick::Enter(1)),
            "…for Enter to take, as `y` Enter always did"
        );
        assert_eq!(
            press_at(&menu, &[plain(Left), plain(Tab), plain(Enter)]),
            Step::Chose(Pick::Enter(0)),
            "unbound, ← and Tab do nothing"
        );
        assert_eq!(press_at(&menu, &[plain(Esc)]), Step::Cancel);

        let yes_no = ["Yes", "No"].map(String::from).to_vec();
        let confirm = Ask {
            options: &yes_no,
            cursor: 1,
            ..Ask::default()
        };
        assert_eq!(
            press_at(&confirm, &[plain(Enter)]),
            Step::Chose(Pick::Enter(1)),
            "the cursor starts on the default"
        );
        assert_eq!(
            press_at(&confirm, &[plain(Char('y')), plain(Enter)]),
            Step::Chose(Pick::Enter(0))
        );
    }

    /// A menu's box as text, styles dropped, over an empty picker.
    fn boxed_text(ask: &Ask, keys: &[KeyEvent], size: (usize, usize)) -> Vec<String> {
        let mut list = List::new(ask);
        for key in keys {
            apply(&mut list, *key, 10);
        }
        menu(&list, size)
            .0
            .iter()
            .map(|line| line.iter().map(|span| span.content().as_str()).collect())
            .collect()
    }

    #[test]
    fn a_menu_is_a_box_over_the_picker_with_its_keys_on_the_last_line() {
        let options = ["Join", "Kill"].map(String::from).to_vec();
        let menu = Ask {
            message: "ada/VBSN-4-picker",
            options: &options,
            keys: "← to go back",
            arrows: true,
            ..Ask::default()
        };

        let pad = " ".repeat(8);
        assert_eq!(
            boxed_text(&menu, &[plain(KeyCode::Down)], (40, 8)),
            [
                String::new(),
                format!("{pad}┌─ ada/VBSN-4-picker ─┐"),
                format!("{pad}│   1. Join           │"),
                format!("{pad}│ ❯ 2. Kill           │"),
                format!("{pad}└─────────────────────┘"),
                String::new(),
                String::new(),
                "Enter to select · ↑/↓ to navigate · 1-2 ".to_string(),
            ]
        );
    }

    #[test]
    fn the_filter_cursor_sits_right_after_what_was_typed() {
        let options = options();
        let ask = Ask {
            message: "Open",
            ..picker(&options)
        };
        let mut list = List::new(&ask);
        for key in typed("api") {
            apply(&mut list, key, 10);
        }
        let (lines, column) = frame(&list, 80, 10);
        let question: String = lines[1]
            .iter()
            .map(|span| span.content().as_str())
            .collect();
        assert_eq!(question, "│ Open  api");
        assert_eq!(
            column,
            Some(question.chars().count()),
            "not two cells past it"
        );
    }

    #[test]
    fn notes_sit_in_the_box_above_the_options() {
        let options = ["Yes", "No"].map(String::from).to_vec();
        let notes = [
            "api-VBSN-1 has uncommitted changes".to_string(),
            "fatal: locked\nuse 'remove -f -f'".to_string(),
        ];
        let ask = Ask {
            message: "Remove api-VBSN-2?",
            options: &options,
            notes: &notes,
            ..Ask::default()
        };
        let body: Vec<String> = boxed_text(&ask, &[], (30, 14))
            .iter()
            .filter_map(|line| {
                let inside = line.split_once("│ ")?.1;
                Some(inside.rsplit_once(" │")?.0.trim_end().to_string())
            })
            .collect();
        assert_eq!(
            body,
            [
                "api-VBSN-1 has",
                "uncommitted changes",
                "fatal: locked",
                "use 'remove -f -f'",
                "",
                "❯ 1. Yes",
                "  2. No",
            ],
            "wrapped to the box, not cut, and git's lines kept apart"
        );
    }

    #[test]
    fn the_row_under_the_cursor_is_previewed_beside_the_list() {
        let options = ["one", "two"].map(String::from).to_vec();
        let previews = [vec!["first".to_string()], Vec::new()];
        let ask = Ask {
            message: "Open",
            previews: &previews,
            ..picker(&options)
        };

        let window = text(&ask, &[], 120);
        let (left, inner) = split(120);
        assert_eq!((left, inner), (80, 34));
        assert_eq!(window[1], "│ Open  type to filter");
        assert_eq!(
            window[3],
            format!("{:<80}  ┌{}┐", "❯ one", "─".repeat(36)),
            "the panel starts where the list's width ends"
        );
        assert_eq!(window[4], format!("{:<80}  │ {:<34} │", "  two", "first"));
        assert_eq!(window[5], format!("{:<80}  └{}┘", "", "─".repeat(36)));

        assert_eq!(
            text(&ask, &[], 99)[3],
            "❯ one",
            "nor has a terminal too narrow for one"
        );
    }

    #[test]
    fn a_heading_is_passed_over_and_filtered_away() {
        use KeyCode::{Down, End, Enter, Home, PageDown, PageUp, Up};
        let options = ["── sessions", "one", "── projects", "two", "three"]
            .map(String::from)
            .to_vec();
        let previews = [
            vec![],
            vec!["1".into()],
            vec![],
            vec!["2".into()],
            vec!["3".into()],
        ];
        let ask = Ask {
            previews: &previews,
            ..picker(&options)
        };

        assert_eq!(
            press_at(&ask, &[plain(Enter)]),
            Step::Chose(Pick::Enter(1)),
            "the cursor starts below the first heading"
        );
        assert_eq!(
            press_at(&ask, &[plain(Down), plain(Enter)]),
            Step::Chose(Pick::Enter(3))
        );
        assert_eq!(
            press_at(&ask, &[plain(Up), plain(Enter)]),
            Step::Chose(Pick::Enter(4)),
            "up from the first row still wraps to the last"
        );
        assert_eq!(
            press_at(&ask, &[plain(End), plain(Home), plain(Enter)]),
            Step::Chose(Pick::Enter(1))
        );
        assert_eq!(
            press_at(&ask, &[plain(PageDown), plain(PageUp), plain(Enter)]),
            Step::Chose(Pick::Enter(1)),
            "a page that lands on a heading moves off it"
        );
        let mut keys = typed("o");
        keys.push(plain(Enter));
        assert_eq!(
            press_at(&ask, &keys),
            Step::Chose(Pick::Enter(1)),
            "`sessions` and `projects` have an o, and are not rows"
        );

        let window = text(&ask, &typed("o"), 99);
        assert!(
            !window
                .iter()
                .any(|line| line.contains("sessions") || line.contains("projects")),
            "{window:?}"
        );
    }

    #[test]
    fn the_page_fills_the_terminal_and_follows_the_cursor() {
        assert_eq!(page_size(50, 0), 45, "every row but the window's five");
        assert_eq!(page_size(24, 2), 17, "and the notes'");
        assert_eq!(
            page_size(0, 0),
            1,
            "a terminal with no height does not underflow"
        );
        let window = text(&picker(&options()), &[], 80);
        assert_eq!(window.len(), 15, "a page of ten fills fifteen rows");
        assert!(
            window[14].starts_with("Enter to select"),
            "the keys at the bottom"
        );

        assert_eq!(top(0, 30, 10), 0);
        assert_eq!(top(4, 30, 10), 0, "the first half page does not scroll");
        assert_eq!(top(12, 30, 10), 7, "the cursor is held mid-page");
        assert_eq!(top(29, 30, 10), 20, "the last page is full, not half empty");
        assert_eq!(top(3, 4, 10), 0, "a list shorter than a page never scrolls");
    }
}
