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

impl Default for PgnAnnotation {
    fn default() -> Self {
        PgnAnnotation::Unknown
    }
}

/// Returns the underlying i8 discriminant of the enum (works for arbitrary stored values).
#[inline]
pub fn annotation_as_i8(a: &PgnAnnotation) -> i8 {
    // SAFETY: PgnAnnotation is #[repr(i8)] so its representation is a single i8.
    unsafe { *(a as *const PgnAnnotation as *const i8) }
}

impl From<i8> for PgnAnnotation {
    fn from(value: i8) -> Self {
        // Map valid enum values directly. For NAG values outside the standard range,
        // use unsafe ptr::write to bypass rustc's validity check on enum construction.
        match value {
            -1 => PgnAnnotation::Unknown,
            0 => PgnAnnotation::Null,
            1 => PgnAnnotation::GoodMove,
            2 => PgnAnnotation::Mistake,
            3 => PgnAnnotation::BrilliantMove,
            4 => PgnAnnotation::Blunder,
            5 => PgnAnnotation::InterestingMove,
            6 => PgnAnnotation::DubiousMove,
            _ => unsafe {
                // SAFETY: PgnAnnotation is #[repr(i8)] so its representation is a single i8.
                // We deliberately store an out-of-range NAG value here. The value is only
                // read back via `annotation_as_i8` (raw byte read); never matched as enum.
                let mut buf = std::mem::MaybeUninit::<PgnAnnotation>::uninit();
                std::ptr::write(buf.as_mut_ptr() as *mut i8, value);
                buf.assume_init()
            },
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
            assert!(cursor < bytes.len() && bytes[cursor].is_ascii_digit());

            let start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            let num_str = std::str::from_utf8(&bytes[start..cursor]).unwrap();
            let num: i64 = num_str.parse().unwrap_or(-1);

            // Map standard NAG values (0..=6) to known variants; everything else is Unknown.
            annotation = match num {
                0 => PgnAnnotation::Null,
                1 => PgnAnnotation::GoodMove,
                2 => PgnAnnotation::Mistake,
                3 => PgnAnnotation::BrilliantMove,
                4 => PgnAnnotation::Blunder,
                5 => PgnAnnotation::InterestingMove,
                6 => PgnAnnotation::DubiousMove,
                _ => PgnAnnotation::Unknown,
            };

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
        // Use raw value to avoid UB on out-of-range NAG values.
        let v = annotation_as_i8(self);
        match v {
            1 => f.write_str("!"),
            2 => f.write_str("?"),
            3 => f.write_str("!!"),
            4 => f.write_str("??"),
            5 => f.write_str("!?"),
            6 => f.write_str("?!"),
            _ => Ok(()), // Unknown, Null and NAG ints have no symbolic form
        }
    }
}

impl From<&str> for PgnAnnotation {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        PgnAnnotation::pgn_annotation_from_string(s, &mut consumed)
    }
}
