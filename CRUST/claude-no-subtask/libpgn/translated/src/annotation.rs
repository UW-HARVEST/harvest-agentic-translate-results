use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgnAnnotation {
    Unknown,
    Null,            // 0
    GoodMove,        // 1: !
    Mistake,         // 2: ?
    BrilliantMove,   // 3: !!
    Blunder,         // 4: ??
    InterestingMove, // 5: !?
    DubiousMove,     // 6: ?!
    /// Catch-all for NAG values >= 7 (e.g. $9)
    OtherNag(i32),
}

impl PgnAnnotation {
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
            PgnAnnotation::OtherNag(v) => *v,
        }
    }
}

impl From<i8> for PgnAnnotation {
    fn from(value: i8) -> Self {
        Self::from(value as i32)
    }
}

impl From<i32> for PgnAnnotation {
    fn from(value: i32) -> Self {
        match value {
            -1 => PgnAnnotation::Unknown,
            0 => PgnAnnotation::Null,
            1 => PgnAnnotation::GoodMove,
            2 => PgnAnnotation::Mistake,
            3 => PgnAnnotation::BrilliantMove,
            4 => PgnAnnotation::Blunder,
            5 => PgnAnnotation::InterestingMove,
            6 => PgnAnnotation::DubiousMove,
            v => PgnAnnotation::OtherNag(v),
        }
    }
}

impl PgnAnnotation {
    /// Parses a PGN NAG annotation from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = PgnAnnotation::Unknown;
        let mut cursor = 0usize;

        if cursor >= bytes.len() || bytes[cursor] != b'$' {
            return annotation;
        }

        while cursor < bytes.len() && bytes[cursor] == b'$' {
            cursor += 1;
            assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());

            let start = cursor;
            while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                cursor += 1;
            }
            let num_str = &str[start..cursor];
            let num: i32 = num_str.parse().unwrap_or(-1);
            // For parsing, treat values that are not in range 0-6 as Unknown.
            annotation = if (0..=6).contains(&num) {
                PgnAnnotation::from(num)
            } else {
                PgnAnnotation::Unknown
            };

            crate::utils::cursor::pgn_cursor_skip_whitespace(str, &mut cursor);
        }
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

    /// Writes a string representation appropriate for "in-line" annotation output
    /// (without the leading space). Returns the number of bytes written.
    pub fn pgn_annotation_to_string(&self, dest: &mut String) -> usize {
        match self {
            PgnAnnotation::Unknown => 0,
            PgnAnnotation::Null => {
                let s = "$0";
                dest.push_str(s);
                s.len()
            }
            PgnAnnotation::GoodMove => { dest.push('!'); 1 }
            PgnAnnotation::Mistake => { dest.push('?'); 1 }
            PgnAnnotation::BrilliantMove => { dest.push_str("!!"); 2 }
            PgnAnnotation::Blunder => { dest.push_str("??"); 2 }
            PgnAnnotation::InterestingMove => { dest.push_str("!?"); 2 }
            PgnAnnotation::DubiousMove => { dest.push_str("?!"); 2 }
            PgnAnnotation::OtherNag(v) => {
                let s = format!("${}", v);
                let n = s.len();
                dest.push_str(&s);
                n
            }
        }
    }
}

impl Display for PgnAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PgnAnnotation::Unknown => Ok(()),
            PgnAnnotation::Null => write!(f, "$0"),
            PgnAnnotation::GoodMove => f.write_str("!"),
            PgnAnnotation::Mistake => f.write_str("?"),
            PgnAnnotation::BrilliantMove => f.write_str("!!"),
            PgnAnnotation::Blunder => f.write_str("??"),
            PgnAnnotation::InterestingMove => f.write_str("!?"),
            PgnAnnotation::DubiousMove => f.write_str("?!"),
            PgnAnnotation::OtherNag(v) => write!(f, "${}", v),
        }
    }
}

impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
