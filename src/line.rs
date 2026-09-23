//! The text prompt: one line, edited with the keys a shell taught you.
//!
//! inquire's text input binds arrows, Home, End and Backspace and nothing
//! else, and its key map is a hardcoded match with no hook, so `C-a` on a
//! pre-filled answer types an `a`. This is the same prompt with readline's
//! bindings, on the crossterm inquire itself reads keys with.
//!
//! A pure [`apply`] holds every binding and no I/O; the loop around it only
//! reads keys and redraws.

use crate::host::Aborted;
use anyhow::Result;
use crossterm::cursor::MoveToColumn;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::queue;
use crossterm::style::{Print, PrintStyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use std::io::{self, Write};

/// The line being edited. The cursor is an index into `chars`, `0..=len`.
/// The picker's filter is one too.
#[derive(Default)]
pub(crate) struct Line {
    pub(crate) chars: Vec<char>,
    pub(crate) cursor: usize,
}

/// What a key did to the prompt, rather than to the line.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Continue,
    Submit,
    Cancel,
}

/// Every binding, and nothing else: no terminal, no I/O.
pub(crate) fn apply(line: &mut Line, key: KeyEvent) -> Step {
    // A terminal that reports releases and repeats reports each of them once
    // too often for a text field.
    if key.kind != KeyEventKind::Press {
        return Step::Continue;
    }
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let end = line.chars.len();

    match (key.code, ctrl, alt) {
        (KeyCode::Enter, ..) => return Step::Submit,
        (KeyCode::Esc, ..) | (KeyCode::Char('c' | 'g'), true, false) => return Step::Cancel,
        // `C-d` on an empty line is end of input, as it is in every shell.
        (KeyCode::Char('d'), true, false) if line.chars.is_empty() => return Step::Cancel,

        (KeyCode::Home, ..) | (KeyCode::Char('a'), true, false) => line.cursor = 0,
        (KeyCode::End, ..) | (KeyCode::Char('e'), true, false) => line.cursor = end,
        (KeyCode::Left, false, false) | (KeyCode::Char('b'), true, false) => {
            line.cursor = line.cursor.saturating_sub(1)
        }
        (KeyCode::Right, false, false) | (KeyCode::Char('f'), true, false) => {
            line.cursor = (line.cursor + 1).min(end)
        }
        (KeyCode::Left, true, false) | (KeyCode::Char('b'), false, true) => {
            line.cursor = word_left(&line.chars, line.cursor)
        }
        (KeyCode::Right, true, false) | (KeyCode::Char('f'), false, true) => {
            line.cursor = word_right(&line.chars, line.cursor)
        }

        (KeyCode::Backspace, false, false) | (KeyCode::Char('h'), true, false) => {
            if line.cursor > 0 {
                line.cursor -= 1;
                line.chars.remove(line.cursor);
            }
        }
        (KeyCode::Delete, ..) | (KeyCode::Char('d'), true, false) => {
            if line.cursor < end {
                line.chars.remove(line.cursor);
            }
        }
        (KeyCode::Backspace, false, true) | (KeyCode::Char('w'), true, false) => {
            let start = word_left(&line.chars, line.cursor);
            line.chars.drain(start..line.cursor);
            line.cursor = start;
        }
        (KeyCode::Char('d'), false, true) => {
            let stop = word_right(&line.chars, line.cursor);
            line.chars.drain(line.cursor..stop);
        }
        (KeyCode::Char('k'), true, false) => line.chars.truncate(line.cursor),
        (KeyCode::Char('u'), true, false) => {
            line.chars.drain(..line.cursor);
            line.cursor = 0;
        }

        (KeyCode::Char(c), false, false) => {
            line.chars.insert(line.cursor, c);
            line.cursor += 1;
        }
        // An unbound chord is not text and is never typed as such.
        _ => {}
    }
    Step::Continue
}

/// The start of the word left of `at`: the separators, then the word. A word
/// is a run of alphanumerics, which is what inquire's own word motion means by
/// one — the picker's filter line and this line should not disagree.
fn word_left(chars: &[char], at: usize) -> usize {
    let mut at = at;
    while at > 0 && !chars[at - 1].is_alphanumeric() {
        at -= 1;
    }
    while at > 0 && chars[at - 1].is_alphanumeric() {
        at -= 1;
    }
    at
}

/// The end of the word right of `at`, the mirror of [`word_left`].
fn word_right(chars: &[char], at: usize) -> usize {
    let mut at = at;
    while at < chars.len() && !chars[at].is_alphanumeric() {
        at += 1;
    }
    while at < chars.len() && chars[at].is_alphanumeric() {
        at += 1;
    }
    at
}

/// Free text, pre-filled with an editable `initial`, or [`Aborted`].
pub fn edit(message: &str, initial: &str) -> Result<String> {
    enable_raw_mode()?;
    let answer = read(message, initial);
    let _ = disable_raw_mode();
    answer
}

fn read(message: &str, initial: &str) -> Result<String> {
    let mut line = Line {
        chars: initial.chars().collect(),
        cursor: initial.chars().count(),
    };

    loop {
        render(message, &line, "?")?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        let step = match apply(&mut line, key) {
            Step::Continue => continue,
            step => step,
        };

        // An answered prompt takes inquire's `>`; an abandoned one keeps its
        // `?`, since nothing was answered and `main` says nothing either.
        let answered = step == Step::Submit;
        render(message, &line, if answered { ">" } else { "?" })?;
        let mut out = io::stdout().lock();
        write!(out, "\r\n")?;
        out.flush()?;

        return match answered {
            true => Ok(line.chars.iter().collect()),
            false => Err(Aborted.into()),
        };
    }
}

/// `? <message> <line>`, the mark in the light green inquire uses for it and
/// honouring `NO_COLOR` as inquire does, with the cursor where the line says.
fn render(message: &str, line: &Line, mark: &str) -> Result<()> {
    let mark = match std::env::var("NO_COLOR") {
        Ok(_) => mark.stylize(),
        Err(_) => mark.green(),
    };
    let text: String = line.chars.iter().collect();
    // ponytail: a line wider than the terminal wraps and the cursor then lands
    // on the wrong row. A scrolling window, if the answers ever get that long.
    let column = "? ".len() + message.chars().count() + 1 + line.cursor;

    let mut out = io::stdout().lock();
    queue!(
        out,
        MoveToColumn(0),
        Clear(ClearType::UntilNewLine),
        PrintStyledContent(mark),
        Print(format!(" {message} {text}")),
        MoveToColumn(column as u16),
    )?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn alt(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
    }

    fn plain(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// The line after those keys, with `|` written where the cursor ended up.
    fn press(start: &str, keys: &[KeyEvent]) -> String {
        let mut line = Line {
            chars: start.chars().collect(),
            cursor: start.chars().count(),
        };
        for key in keys {
            apply(&mut line, *key);
        }
        let mut chars = line.chars;
        chars.insert(line.cursor, '|');
        chars.into_iter().collect()
    }

    /// A session name is the answer these keys exist to edit, and it has the
    /// separators that make word motion worth testing.
    const NAME: &str = "ada/VBSN-1-init";

    #[test]
    fn the_emacs_keys_move_by_character_and_by_word() {
        assert_eq!(press(NAME, &[ctrl('a')]), "|ada/VBSN-1-init", "C-a");
        assert_eq!(press(NAME, &[ctrl('a'), ctrl('e')]), "ada/VBSN-1-init|");
        assert_eq!(press(NAME, &[ctrl('b'), ctrl('b')]), "ada/VBSN-1-in|it");
        assert_eq!(press(NAME, &[ctrl('a'), ctrl('f')]), "a|da/VBSN-1-init");
        assert_eq!(press(NAME, &[alt('b')]), "ada/VBSN-1-|init", "M-b");
        assert_eq!(press(NAME, &[alt('b'), alt('b')]), "ada/VBSN-|1-init");
        assert_eq!(
            press(NAME, &[ctrl('a'), alt('f'), alt('f')]),
            "ada/VBSN|-1-init",
            "M-f crosses the separator to the end of the next word"
        );
    }

    #[test]
    fn the_emacs_keys_kill_by_word_and_to_each_end() {
        assert_eq!(press(NAME, &[ctrl('w')]), "ada/VBSN-1-|", "C-w");
        assert_eq!(press(NAME, &[ctrl('a'), alt('d')]), "|/VBSN-1-init", "M-d");
        assert_eq!(press(NAME, &[alt('b'), ctrl('k')]), "ada/VBSN-1-|", "C-k");
        assert_eq!(press(NAME, &[alt('b'), ctrl('u')]), "|init", "C-u");
        assert_eq!(press(NAME, &[ctrl('a'), ctrl('d')]), "|da/VBSN-1-init");
        assert_eq!(
            press(NAME, &[ctrl('h')]),
            "ada/VBSN-1-ini|",
            "C-h, not an h"
        );
        assert_eq!(press(NAME, &[ctrl('z')]), NAME.to_owned() + "|", "unbound");
    }

    #[test]
    fn the_keys_inquire_already_bound_still_do_what_they_did() {
        use KeyCode::{Backspace, Delete, End, Home, Left, Right};

        assert_eq!(press("abc", &[plain(Backspace)]), "ab|");
        assert_eq!(press("abc", &[plain(Home), plain(Delete)]), "|bc");
        assert_eq!(
            press("abc", &[plain(Left), plain(KeyCode::Char('X'))]),
            "abX|c"
        );
        assert_eq!(
            press("abc", &[plain(Home), plain(Right), plain(End)]),
            "abc|"
        );
        assert_eq!(
            press("a/b", &[KeyEvent::new(Left, KeyModifiers::CONTROL)]),
            "a/|b",
            "ctrl-arrows move by word, as they did"
        );
    }

    #[test]
    fn enter_submits_and_the_cancel_keys_abort() {
        let step = |start: &str, key: KeyEvent| {
            let mut line = Line {
                chars: start.chars().collect(),
                cursor: 0,
            };
            apply(&mut line, key)
        };

        assert_eq!(step("abc", plain(KeyCode::Enter)), Step::Submit);
        assert_eq!(
            step("", plain(KeyCode::Enter)),
            Step::Submit,
            "an empty line submits as itself"
        );
        assert_eq!(step("abc", plain(KeyCode::Esc)), Step::Cancel);
        assert_eq!(step("abc", ctrl('c')), Step::Cancel);
        assert_eq!(step("abc", ctrl('g')), Step::Cancel);
        assert_eq!(
            step("", ctrl('d')),
            Step::Cancel,
            "C-d on an empty line is end of input"
        );
        assert_eq!(
            step("abc", ctrl('d')),
            Step::Continue,
            "C-d with something to delete deletes it"
        );
    }
}
