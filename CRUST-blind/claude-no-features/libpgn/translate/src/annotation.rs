use std::fmt::Display;

use crate::utils::cursor::{pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace};
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

impl PgnAnnotation {
    /// Returns the integer (NAG) representation of this annotation.
    pub fn as_i32(&self) -> i32 {
        match self {
            PgnAnnotation::Unknown => -1,
            PgnAnnotation::Null => 0,
            PgnAnnotation::GoodMove => 1,
            PgnAnnotation::Mistake => 2,
            PgnAnnotation::BrilliantMove => 3,
            PgnAnnotation::Blunder => 4,
            PgnAnnotation::InterestingMove => 5,
            PgnAnnotation::DubiousMove => 6,
        }
    }
}

impl From<i8> for PgnAnnotation {
    fn from(value: i8) -> Self {
        match value {
            -1 => PgnAnnotation::Unknown,
            0 => PgnAnnotation::Null,
            1 => PgnAnnotation::GoodMove,
            2 => PgnAnnotation::Mistake,
            3 => PgnAnnotation::BrilliantMove,
            4 => PgnAnnotation::Blunder,
            5 => PgnAnnotation::InterestingMove,
            6 => PgnAnnotation::DubiousMove,
            _ => PgnAnnotation::Unknown,
        }
    }
}

impl PgnAnnotation {
    /// Parses a NAG-formatted annotation (`$<num>`) from a string.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;
        let mut cursor: usize = 0;

        if bytes.first().copied() != Some(b'$') {
            return annotation;
        }

        while bytes.get(cursor).copied() == Some(b'$') {
            cursor += 1;
            assert!(
                bytes.get(cursor).map_or(false, |b| b.is_ascii_digit()),
                "expected digit after '$'"
            );

            // Parse a base-10 integer.
            let start = cursor;
            while bytes.get(cursor).map_or(false, |b| b.is_ascii_digit()) {
                cursor += 1;
            }
            let num_str = &str[start..cursor];
            let num: i32 = num_str.parse().expect("parsed digits should be a valid i32");
            annotation = if (-1..=6).contains(&num) {
                PgnAnnotation::from(num as i8)
            } else {
                // For NAG values outside the named-variant range, fall back to
                // `Unknown` (the Rust enum cannot represent arbitrary integers).
                PgnAnnotation::Unknown
            };

            pgn_cursor_skip_whitespace(str, &mut cursor);
        }
        pgn_cursor_revisit_whitespace(str, &mut cursor);

        *consumed += cursor;
        annotation
    }

    /// Parses a `!`/`?`-style annotation from the start of a string.
    pub fn pgn_annotation_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;

        if bytes.is_empty() {
            return annotation;
        }

        let c0 = bytes.first().copied();
        if c0 == Some(b'!') {
            *consumed += 1;
            annotation = PgnAnnotation::GoodMove;
        }
        if c0 == Some(b'?') {
            *consumed += 1;
            annotation = PgnAnnotation::Mistake;
        }

        if bytes.len() < 2 {
            return annotation;
        }

        let c1 = bytes.get(1).copied();
        if c0 == Some(b'!') && c1 == Some(b'!') {
            *consumed += 1;
            annotation = PgnAnnotation::BrilliantMove;
        }
        if c0 == Some(b'!') && c1 == Some(b'?') {
            *consumed += 1;
            annotation = PgnAnnotation::InterestingMove;
        }
        if c0 == Some(b'?') && c1 == Some(b'!') {
            *consumed += 1;
            annotation = PgnAnnotation::DubiousMove;
        }
        if c0 == Some(b'?') && c1 == Some(b'?') {
            *consumed += 1;
            annotation = PgnAnnotation::Blunder;
        }

        annotation
    }
}

impl Display for PgnAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PgnAnnotation::Unknown => Ok(()),
            PgnAnnotation::Null => Ok(()),
            PgnAnnotation::GoodMove => f.write_str("!"),
            PgnAnnotation::Mistake => f.write_str("?"),
            PgnAnnotation::BrilliantMove => f.write_str("!!"),
            PgnAnnotation::Blunder => f.write_str("??"),
            PgnAnnotation::InterestingMove => f.write_str("!?"),
            PgnAnnotation::DubiousMove => f.write_str("?!"),
        }
    }
}

impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
