use std::fmt::Display;
use crate::utils::cursor::{pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace};

/// PgnAnnotation mirrors the C `pgn_annotation_t` enum but, like C, can hold
/// arbitrary integer NAG values (e.g. `$19`, `$69`, `$420`). The well-known
/// values are exposed as associated constants so existing code can keep using
/// `PgnAnnotation::GoodMove`, `PgnAnnotation::Unknown`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PgnAnnotation(pub i32);

#[allow(non_upper_case_globals)]
impl PgnAnnotation {
    pub const Unknown: PgnAnnotation = PgnAnnotation(-1);
    pub const Null: PgnAnnotation = PgnAnnotation(0);
    pub const GoodMove: PgnAnnotation = PgnAnnotation(1);
    pub const Mistake: PgnAnnotation = PgnAnnotation(2);
    pub const BrilliantMove: PgnAnnotation = PgnAnnotation(3);
    pub const Blunder: PgnAnnotation = PgnAnnotation(4);
    pub const InterestingMove: PgnAnnotation = PgnAnnotation(5);
    pub const DubiousMove: PgnAnnotation = PgnAnnotation(6);
}

impl From<i32> for PgnAnnotation {
    fn from(value: i32) -> Self {
        PgnAnnotation(value)
    }
}

impl From<i8> for PgnAnnotation {
    fn from(value: i8) -> Self {
        PgnAnnotation(value as i32)
    }
}

impl PgnAnnotation {
    /// Parses a NAG annotation (e.g. `$19`) from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;
        let mut cursor = 0usize;

        if cursor >= bytes.len() || bytes[cursor] != b'$' {
            return annotation;
        }

        while cursor < bytes.len() && bytes[cursor] == b'$' {
            cursor += 1;
            // expect a digit
            debug_assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());

            // parse number
            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                cursor += 1;
            }
            let num_str = &str[start..cursor];
            if let Ok(num) = num_str.parse::<i64>() {
                annotation = PgnAnnotation(num as i32);
            }

            pgn_cursor_skip_whitespace(str, &mut cursor);
        }
        pgn_cursor_revisit_whitespace(str, &mut cursor);

        *consumed += cursor;
        annotation
    }

    /// Parses a PGN annotation (`!`, `?`, `!!`, `??`, `!?`, `?!`) from a string.
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

    /// Writes the annotation into a string buffer the same way `pgn_annotation_to_string` does
    /// in C, returning the number of bytes written. Used by Display.
    pub fn to_dest(&self, dest: &mut String) -> usize {
        match *self {
            PgnAnnotation::Unknown => 0,
            PgnAnnotation::Null => 0, /* C `case PGN_ANNOTATION_NULL: break;` falls through */
            PgnAnnotation::GoodMove => { dest.push('!'); 1 }
            PgnAnnotation::Mistake => { dest.push('?'); 1 }
            PgnAnnotation::BrilliantMove => { dest.push_str("!!"); 2 }
            PgnAnnotation::Blunder => { dest.push_str("??"); 2 }
            PgnAnnotation::InterestingMove => { dest.push_str("!?"); 2 }
            PgnAnnotation::DubiousMove => { dest.push_str("?!"); 2 }
            PgnAnnotation(n) => {
                let s = format!("${}", n);
                let len = s.len();
                dest.push_str(&s);
                len
            }
        }
    }
}

impl Display for PgnAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PgnAnnotation::Unknown => Ok(()),
            PgnAnnotation::Null => write!(f, "$0"),
            PgnAnnotation::GoodMove => f.write_str("!"),
            PgnAnnotation::Mistake => f.write_str("?"),
            PgnAnnotation::BrilliantMove => f.write_str("!!"),
            PgnAnnotation::Blunder => f.write_str("??"),
            PgnAnnotation::InterestingMove => f.write_str("!?"),
            PgnAnnotation::DubiousMove => f.write_str("?!"),
            PgnAnnotation(n) => write!(f, "${}", n),
        }
    }
}

impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
