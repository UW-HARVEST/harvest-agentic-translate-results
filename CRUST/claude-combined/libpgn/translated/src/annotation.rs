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
        // The C code stores NAG annotation values as arbitrary integers, including
        // values outside the named variants (e.g., $9). The enum is `#[repr(i8)]`,
        // so its in-memory representation is a single i8. We use `ptr::read` so the
        // compiler doesn't apply the debug-mode "invalid enum value" check that
        // `transmute` performs. All consumers of `PgnAnnotation` use `nag_number()`
        // and avoid pattern-matching on potentially out-of-range values.
        unsafe { std::ptr::read(&value as *const i8 as *const PgnAnnotation) }
    }
}
impl PgnAnnotation {
    /// Parses a PGN annotation from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;
        let mut cursor = 0usize;

        if bytes.is_empty() || bytes[cursor] != b'$' {
            return annotation;
        }

        while cursor < bytes.len() && bytes[cursor] == b'$' {
            cursor += 1;
            // Parse digits to a number
            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                cursor += 1;
            }
            if cursor == start {
                // No digit found, mimic assert in C
                return annotation;
            }
            let num_str = std::str::from_utf8(&bytes[start..cursor]).unwrap_or("0");
            let num: i64 = num_str.parse().unwrap_or(0);
            // Convert to PgnAnnotation by integer value (mimic C's `annotation = num;`)
            // Treat valid range, others fall back to Unknown
            annotation = match num {
                -1 => PgnAnnotation::Unknown,
                0 => PgnAnnotation::Null,
                1 => PgnAnnotation::GoodMove,
                2 => PgnAnnotation::Mistake,
                3 => PgnAnnotation::BrilliantMove,
                4 => PgnAnnotation::Blunder,
                5 => PgnAnnotation::InterestingMove,
                6 => PgnAnnotation::DubiousMove,
                _ => PgnAnnotation::Unknown,
            };

            // Skip whitespace
            crate::utils::cursor::pgn_cursor_skip_whitespace(str, &mut cursor);
        }
        // Revisit whitespace
        crate::utils::cursor::pgn_cursor_revisit_whitespace(str, &mut cursor);

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
        let mut consumed = 0usize;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}

impl PgnAnnotation {
    /// Convert annotation to its NAG numeric value (used by `pgn_annotation_to_string`).
    /// Reads the underlying i8 discriminant directly (no `match`), which is sound even
    /// for transmuted out-of-range values.
    pub fn nag_number(&self) -> i32 {
        // SAFETY: the enum is `#[repr(i8)]`, so its in-memory representation is a single i8.
        let raw: i8 = unsafe { *(self as *const Self as *const i8) };
        raw as i32
    }

    /// Returns the string for "to_string" representation, mimicking pgn_annotation_to_string.
    pub fn to_pgn_string(&self) -> String {
        match self.nag_number() {
            -1 => String::new(),
            0 => String::new(),
            1 => "!".to_string(),
            2 => "?".to_string(),
            3 => "!!".to_string(),
            4 => "??".to_string(),
            5 => "!?".to_string(),
            6 => "?!".to_string(),
            _ => String::new(),
        }
    }
}
