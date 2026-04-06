use std::fmt::Display;
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
            // Store raw NAG value - mirrors C behavior where enum is just an int
            _ => {
                let mut result = PgnAnnotation::Unknown;
                unsafe {
                    let ptr = &mut result as *mut PgnAnnotation as *mut i8;
                    std::ptr::write(ptr, value);
                }
                result
            }
        }
    }
}
impl PgnAnnotation {
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
            assert!(cursor < bytes.len() && bytes[cursor].is_ascii_digit());

            let start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            let num: i64 = str[start..cursor].parse().unwrap();
            // For NAG parsing, values 0-6 map to known variants, others to Unknown
            // (differs from From<i8> which preserves raw values for direct construction)
            if num >= 0 && num <= 6 {
                annotation = PgnAnnotation::from(num as i8);
            } else {
                annotation = PgnAnnotation::Unknown;
            }

            // skip whitespace
            while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_whitespace() {
                cursor += 1;
            }
        }
        // revisit whitespace
        while cursor > 0 && (bytes[cursor - 1] as char).is_ascii_whitespace() {
            cursor -= 1;
        }

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
            PgnAnnotation::Null => Ok(()), // Null falls through to $N in C's annotation_to_string
            PgnAnnotation::GoodMove => write!(f, "!"),
            PgnAnnotation::Mistake => write!(f, "?"),
            PgnAnnotation::BrilliantMove => write!(f, "!!"),
            PgnAnnotation::Blunder => write!(f, "??"),
            PgnAnnotation::InterestingMove => write!(f, "!?"),
            PgnAnnotation::DubiousMove => write!(f, "?!"),
        }
    }
}
impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
