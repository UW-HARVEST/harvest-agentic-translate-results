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
    /// Parses a PGN annotation from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;
        let mut cursor: usize = 0;

        if cursor >= bytes.len() || bytes[cursor] != b'$' {
            return annotation;
        }

        while cursor < bytes.len() && bytes[cursor] == b'$' {
            cursor += 1;
            assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());

            // Parse a number starting at cursor.
            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                cursor += 1;
            }
            let num: i32 = str[start..cursor].parse().unwrap_or(0);
            annotation = PgnAnnotation::from(num as i8);

            pgn_cursor_skip_whitespace(str, &mut cursor);
        }
        pgn_cursor_revisit_whitespace(str, &mut cursor);

        *consumed += cursor;
        annotation
    }

    /// Parses a PGN annotation from a string (wrapper around the inner function).
    pub fn pgn_annotation_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;

        if bytes.is_empty() {
            return annotation;
        }

        if bytes[0] == b'!' {
            *consumed += 1;
            annotation = PgnAnnotation::GoodMove;
        }

        if bytes[0] == b'?' {
            *consumed += 1;
            annotation = PgnAnnotation::Mistake;
        }

        if bytes.len() < 2 {
            return annotation;
        }

        if bytes[0] == b'!' && bytes[1] == b'!' {
            *consumed += 1;
            annotation = PgnAnnotation::BrilliantMove;
        }

        if bytes[0] == b'!' && bytes[1] == b'?' {
            *consumed += 1;
            annotation = PgnAnnotation::InterestingMove;
        }

        if bytes[0] == b'?' && bytes[1] == b'!' {
            *consumed += 1;
            annotation = PgnAnnotation::DubiousMove;
        }

        if bytes[0] == b'?' && bytes[1] == b'?' {
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
        let mut consumed = 0usize;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
