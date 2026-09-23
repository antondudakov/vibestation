//! Every list prompt, drawn as the window Claude Code asks its questions in: a
//! rule, the question, the options with `❯` on the one under the cursor and a
//! panel beside them previewing it, a rule, and the keys.
//!
//! Two kinds of list. The picker filters as you type, on [`crate::line`]'s
//! line and skim's scorer, since a ticket number is how you reach a worktree.
//! Everything else — a row's menu, the branch question, yes or no — is a
//! handful of options, so they are numbered and a digit chooses.
//!
//! A pure [`apply`] holds every binding and a pure [`frame`] every line drawn;
//! the loop around them only reads keys and prints.

use crate::host::{Aborted, Pick};
use crate::line::{self, paint, Line};
use anyhow::Result;
use crossterm::cursor::{Hide, MoveToColumn, MoveUp, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Color, Print, PrintStyledContent, StyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{execute, queue};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::cmp::Reverse;
use std::io::{self, Write};

/// A list, as the host asks for one.
#[derive(Default)]
pub struct Ask<'a> {
    pub message: &'a str,
    pub options: &'a [String],
    /// Beside the list, a panel for the option under the cursor. Empty for no
    /// panel, as is any one option's.
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
        List {
            ask,
            filter: Line::default(),
            shown: (0..ask.options.len()).collect(),
            cursor: ask.cursor.min(ask.options.len().saturating_sub(1)),
            // inquire's scorer, so a filter finds what it always found.
            matcher: SkimMatcherV2::default().ignore_case(),
        }
    }

    fn highlighted(&self) -> Option<usize> {
        self.shown.get(self.cursor).copied()
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

/// How many rows the list may show: the terminal less the window's five lines
/// — two rules, the question, the gap under it and the keys — and two rows of
/// margin, so opening the list never scrolls its own top off the screen.
/// Floored at five — a very short terminal still needs enough rows to be a
/// list.
fn page_size(height: usize) -> usize {
    height.saturating_sub(7).max(5)
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

/// Choose one of the options in a terminal of `(width, height)`, or
/// [`Aborted`].
pub fn choose(ask: &Ask, (width, height): (usize, usize)) -> Result<Pick> {
    enable_raw_mode()?;
    let picked = read(ask, width, page_size(height));
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), Show);
    picked
}

fn read(ask: &Ask, width: usize, page: usize) -> Result<Pick> {
    let mut list = List::new(ask);
    let mut out = io::stdout().lock();
    let mut drawn = false;

    loop {
        let (lines, column) = frame(&list, width, page);
        render(&mut out, &lines, column, drawn)?;
        drawn = true;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let step = match apply(&mut list, key, page) {
            Step::Continue => continue,
            step => step,
        };

        // Up from the question to the rule, and the window is wiped.
        queue!(
            out,
            MoveUp(1),
            MoveToColumn(0),
            Clear(ClearType::FromCursorDown)
        )?;
        let Step::Chose(pick) = step else {
            // An abandoned prompt keeps its `?`, as the text prompt's does.
            queue!(out, PrintStyledContent(paint("?", Color::Green)))?;
            queue!(out, Print(format!(" {}\r\n", ask.message)))?;
            out.flush()?;
            return Err(Aborted.into());
        };
        // A chosen row leaves the answered line behind. The arrows and Tab
        // leave nothing: what they open is drawn where the window was.
        if let Pick::Enter(index) = pick {
            queue!(
                out,
                PrintStyledContent(paint(">", Color::Green)),
                Print(format!(" {} ", ask.message)),
                PrintStyledContent(paint(ask.options[index].trim(), Color::Cyan)),
                Print("\r\n"),
            )?;
        }
        out.flush()?;
        return Ok(pick);
    }
}

/// Print the window over the last one and leave the cursor on the question
/// line — in the filter, or hidden when there is none.
fn render(out: &mut impl Write, lines: &Window, column: Option<usize>, drawn: bool) -> Result<()> {
    if drawn {
        queue!(out, MoveUp(1))?;
    }
    queue!(out, MoveToColumn(0), Clear(ClearType::FromCursorDown))?;
    for (at, line) in lines.iter().enumerate() {
        if at > 0 {
            queue!(out, Print("\r\n"))?;
        }
        for span in line {
            queue!(out, PrintStyledContent(span.clone()))?;
        }
    }
    // A window is never fewer than five lines, so this is never `MoveUp(0)`,
    // which terminals take as one.
    queue!(out, MoveUp(lines.len() as u16 - 2))?;
    match column {
        Some(column) => queue!(out, MoveToColumn(column as u16), Show)?,
        None => queue!(out, MoveToColumn(0), Hide)?,
    }
    out.flush()?;
    Ok(())
}

/// One line being written, cut at the terminal's edge: one that wrapped would
/// throw off the move back up, and leave a copy of itself at every redraw.
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

const DIM: Option<Color> = Some(Color::DarkGrey);

/// A window, line by line, each line in its colours.
type Window = Vec<Vec<StyledContent<String>>>;

/// Every line of the window, and the column the cursor sits at on the
/// question line, if it shows at all. Pure, so the layout is testable.
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
    let mut column = None;
    if ask.filter {
        let typed: String = list.filter.chars.iter().collect();
        question.push("  ", None);
        match typed.is_empty() {
            true => question.push("type to filter", DIM),
            false => question.push(&typed, None),
        };
        let at = "│ ".len() + ask.message.chars().count() + 2 + list.filter.cursor;
        column = Some(at.min(width.saturating_sub(1)));
    }
    lines.push(question.spans);
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

    for at in 0..rows.len().max(panel.len()) {
        let mut line = Spans::new(width);
        let text = match rows.get(at) {
            Some(&index) => {
                // `❯` is the cursor; `^` and `v` say the list goes on past the
                // page.
                let marker = match (top + at == list.cursor, at) {
                    (true, _) => "❯",
                    (_, 0) if top > 0 => "^",
                    _ if at + 1 == rows.len() && more_below => "v",
                    _ => " ",
                };
                match ask.filter {
                    true => format!("{marker} {}", ask.options[index]),
                    false => format!("{marker} {}. {}", index + 1, ask.options[index]),
                }
            }
            None => String::new(),
        };
        let here = rows.get(at).is_some() && top + at == list.cursor;
        match panel.get(at) {
            Some(boxed) => {
                line.push(
                    &format!("{:<left$}", cut(&text, left)),
                    here.then_some(Color::Cyan),
                );
                for (text, color) in boxed {
                    line.push(text, *color);
                }
            }
            None => {
                line.push(&text, here.then_some(Color::Cyan));
            }
        }
        lines.push(line.spans);
    }

    lines.push(rule());
    let mut keys = Spans::new(width);
    keys.push(&footer(ask), DIM);
    lines.push(keys.spans);
    (lines, column)
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

/// `text` in lines of at most `width`, broken at the last space that fits,
/// or mid-word when none does.
fn wrap(text: &str, width: usize) -> Vec<String> {
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

fn cut(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
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

    #[test]
    fn the_window_is_a_rule_the_question_the_rows_a_rule_and_the_keys() {
        let options = ["Join", "Kill"].map(String::from).to_vec();
        let menu = Ask {
            message: "ada/VBSN-4-picker",
            options: &options,
            keys: "← to go back",
            arrows: true,
            ..Ask::default()
        };

        assert_eq!(
            text(&menu, &[plain(KeyCode::Down)], 40),
            [
                "─".repeat(40).as_str(),
                "│ ada/VBSN-4-picker",
                "",
                "  1. Join",
                "❯ 2. Kill",
                "─".repeat(40).as_str(),
                "Enter to select · ↑/↓ to navigate · 1-2 ",
            ]
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

        let window = text(&ask, &[plain(KeyCode::Down)], 120);
        assert_eq!(
            &window[3..5],
            ["  one", "❯ two"],
            "a row with nothing to preview has no panel"
        );
        assert_eq!(
            text(&ask, &[], 99)[3],
            "❯ one",
            "nor has a terminal too narrow for one"
        );
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
    fn the_page_fills_the_terminal_and_follows_the_cursor() {
        assert_eq!(page_size(50), 43, "a tall terminal shows 43 rows, not 7");
        assert_eq!(page_size(24), 17);
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
