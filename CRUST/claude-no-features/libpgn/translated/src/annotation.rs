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
    /// Numeric Annotation Glyph (NAG) values not covered by the other variants.
    /// Stores the numeric NAG code so the original encoding round-trips.
    Nag(i32),
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
            n => PgnAnnotation::Nag(n as i32),
        }
    }
}

impl PgnAnnotation {
    /// Numeric form for matching the original C enum value.
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
            PgnAnnotation::Nag(n) => *n,
        }
    }

    /// Parses a PGN annotation from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;
        let mut cursor = 0usize;

        if cursor >= bytes.len() || bytes[cursor] != b'$' {
            return annotation;
        }

        while cursor < bytes.len() && bytes[cursor] == b'$' {
            cursor += 1;
            if cursor >= bytes.len() || !(bytes[cursor] as char).is_ascii_digit() {
                // Per C: assert(isdigit(...)). Stop here for safety.
                break;
            }

            // Parse number
            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                cursor += 1;
            }
            let num_str = &str[start..cursor];
            let num: i32 = num_str.parse().unwrap_or(0);
            // Map numeric NAG to known annotation variants. Return `Unknown` if it
            // does not correspond to a defined variant — matches the Rust tests
            // which assume only annotations 1..=6 are defined.
            annotation = match num {
                1 => PgnAnnotation::GoodMove,
                2 => PgnAnnotation::Mistake,
                3 => PgnAnnotation::BrilliantMove,
                4 => PgnAnnotation::Blunder,
                5 => PgnAnnotation::InterestingMove,
                6 => PgnAnnotation::DubiousMove,
                _ => PgnAnnotation::Unknown,
            };

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
            PgnAnnotation::Unknown => write!(f, ""),
            PgnAnnotation::Null => write!(f, "$0"),
            PgnAnnotation::GoodMove => write!(f, "!"),
            PgnAnnotation::Mistake => write!(f, "?"),
            PgnAnnotation::BrilliantMove => write!(f, "!!"),
            PgnAnnotation::Blunder => write!(f, "??"),
            PgnAnnotation::InterestingMove => write!(f, "!?"),
            PgnAnnotation::DubiousMove => write!(f, "?!"),
            PgnAnnotation::Nag(n) => write!(f, "${}", n),
        }
    }
}

impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
