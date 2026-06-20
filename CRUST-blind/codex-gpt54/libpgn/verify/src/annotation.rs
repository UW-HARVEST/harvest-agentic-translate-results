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
            -1 => Self::Unknown,
            0 => Self::Null,
            1 => Self::GoodMove,
            2 => Self::Mistake,
            3 => Self::BrilliantMove,
            4 => Self::Blunder,
            5 => Self::InterestingMove,
            6 => Self::DubiousMove,
            _ => Self::Unknown,
        }
    }
}
impl PgnAnnotation {
    /// Parses a PGN annotation from a string, consuming characters as needed.
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut cursor = 0;
        let mut annotation = Self::Unknown;

        if !matches!(bytes.get(cursor), Some(b'$')) {
            return annotation;
        }

        while matches!(bytes.get(cursor), Some(b'$')) {
            cursor += 1;
            let start = cursor;
            while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
                cursor += 1;
            }

            if start == cursor {
                break;
            }

            let num = str[start..cursor].parse::<i16>().ok().unwrap_or(-1);
            annotation = PgnAnnotation::from(num as i8);

            while matches!(bytes.get(cursor), Some(b) if (*b as char).is_ascii_whitespace()) {
                cursor += 1;
            }
        }

        while cursor > 0
            && matches!(bytes.get(cursor - 1), Some(b) if (*b as char).is_ascii_whitespace())
        {
            cursor -= 1;
        }

        *consumed += cursor;
        annotation
    }
    /// Parses a PGN annotation from a string (wrapper around the inner function).
    pub fn pgn_annotation_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut annotation = Self::Unknown;

        if bytes.is_empty() {
            return annotation;
        }

        if bytes[0] == b'!' {
            *consumed += 1;
            annotation = Self::GoodMove;
        }

        if bytes[0] == b'?' {
            *consumed += 1;
            annotation = Self::Mistake;
        }

        if bytes.len() < 2 {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Unknown => "",
            Self::Null => "$0",
            Self::GoodMove => "!",
            Self::Mistake => "?",
            Self::BrilliantMove => "!!",
            Self::Blunder => "??",
            Self::InterestingMove => "!?",
            Self::DubiousMove => "?!",
        };
        f.write_str(s)
    }
}
impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        Self::pgn_annotation_from_string(s, &mut consumed)
    }
}
