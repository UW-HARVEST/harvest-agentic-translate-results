use std::fmt::Display;
use std::{cell::Cell, fmt};

thread_local! {
    static RAW_ANNOTATION: Cell<Option<i8>> = const { Cell::new(None) };
}

fn set_raw_annotation(value: Option<i8>) {
    RAW_ANNOTATION.with(|cell| cell.set(value));
}

fn take_raw_annotation() -> Option<i8> {
    RAW_ANNOTATION.with(|cell| {
        let value = cell.get();
        cell.set(None);
        value
    })
}

pub(crate) fn consume_raw_annotation() -> Option<i8> {
    take_raw_annotation()
}

#[repr(i8)] // Ensures the enum has a fixed representation like in C
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgnAnnotation {
    Unknown = -1,
    Null = 0,
    GoodMove,        // !
    Mistake,         // ?
    BrilliantMove,   // !!
    Blunder,         // ??
    InterestingMove, // !?
    DubiousMove,     // ?!
}
impl From<i8> for PgnAnnotation {
    fn from(value: i8) -> Self {
        match value {
            -1 => {
                set_raw_annotation(None);
                Self::Unknown
            }
            0 => {
                set_raw_annotation(None);
                Self::Null
            }
            1 => {
                set_raw_annotation(None);
                Self::GoodMove
            }
            2 => {
                set_raw_annotation(None);
                Self::Mistake
            }
            3 => {
                set_raw_annotation(None);
                Self::BrilliantMove
            }
            4 => {
                set_raw_annotation(None);
                Self::Blunder
            }
            5 => {
                set_raw_annotation(None);
                Self::InterestingMove
            }
            6 => {
                set_raw_annotation(None);
                Self::DubiousMove
            }
            raw => {
                set_raw_annotation(Some(raw));
                Self::Unknown
            }
        }
    }
}
impl PgnAnnotation {
    /// Parses a PGN annotation from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = Self::Unknown;
        let mut cursor = 0usize;

        if bytes.get(cursor) != Some(&b'$') {
            return annotation;
        }

        while bytes.get(cursor) == Some(&b'$') {
            cursor += 1;
            assert!(bytes.get(cursor).is_some_and(u8::is_ascii_digit));

            let start = cursor;
            while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }

            let value = str[start..cursor].parse::<i16>().unwrap_or(-1);
            annotation = match value {
                0 => Self::Null,
                1 => Self::GoodMove,
                2 => Self::Mistake,
                3 => Self::BrilliantMove,
                4 => Self::Blunder,
                5 => Self::InterestingMove,
                6 => Self::DubiousMove,
                _ => Self::Unknown,
            };

            while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
                cursor += 1;
            }
        }
        while cursor > 0 && bytes[cursor - 1].is_ascii_whitespace() {
            cursor -= 1;
        }

        *consumed += cursor;
        annotation
    }
    /// Parses a PGN annotation from a string (wrapper around the inner function).
    pub fn pgn_annotation_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        if bytes.is_empty() {
            return Self::Unknown;
        }

        let mut annotation = Self::Unknown;

        match bytes[0] {
            b'!' => {
                *consumed += 1;
                annotation = Self::GoodMove;
            }
            b'?' => {
                *consumed += 1;
                annotation = Self::Mistake;
            }
            _ => {}
        }

        if bytes.len() == 1 {
            return annotation;
        }

        match (bytes[0], bytes[1]) {
            (b'!', b'!') => {
                *consumed += 1;
                Self::BrilliantMove
            }
            (b'!', b'?') => {
                *consumed += 1;
                Self::InterestingMove
            }
            (b'?', b'!') => {
                *consumed += 1;
                Self::DubiousMove
            }
            (b'?', b'?') => {
                *consumed += 1;
                Self::Blunder
            }
            _ => annotation,
        }
    }
}
impl Display for PgnAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => {
                if let Some(raw) = take_raw_annotation() {
                    write!(f, "${raw}")
                } else {
                    Ok(())
                }
            }
            Self::Null => f.write_str("$0"),
            Self::GoodMove => f.write_str("!"),
            Self::Mistake => f.write_str("?"),
            Self::BrilliantMove => f.write_str("!!"),
            Self::Blunder => f.write_str("??"),
            Self::InterestingMove => f.write_str("!?"),
            Self::DubiousMove => f.write_str("?!"),
        }
    }
}
impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        Self::pgn_annotation_from_string(s, &mut consumed)
    }
}
