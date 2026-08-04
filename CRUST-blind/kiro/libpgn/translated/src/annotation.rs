use std::fmt::Display;
use crate::utils::cursor;

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
            _ => PgnAnnotation::Null, // numeric NAG > 6 stored as Null with value
        }
    }
}

/// Helper to store the raw NAG number for annotations beyond the standard set
/// In the C code, annotation is just an int that can hold any NAG number.
/// We need a way to handle NAG numbers > 6 for to_string.
/// The C code uses the enum value directly as an int, so for NAG $14 it stores 14.
/// We'll use a thread-local or just handle it in the Display impl.
/// Actually, looking at the C code more carefully:
///   - annotation_to_string for values > DUBIOUS_MOVE formats as "$%d"
///   - The enum values map: Unknown=-1, Null=0, Good=1, Mistake=2, Brilliant=3, Blunder=4, Interesting=5, Dubious=6
///   - For NAG $14, annotation = 14 (just the raw int)
/// In Rust we can't store arbitrary i8 in the enum. But the Rust signature returns PgnAnnotation.
/// Looking at the Rust interface, PgnAnnotation has a fixed set. The NAG parsing sets annotation = num.
/// For values > 6, the C code stores the raw number. We need to handle this.
/// 
/// Since the Rust enum is repr(i8), we can transmute... but that's unsafe.
/// The Display impl needs to handle Null specially: it formats as "$%d" using the annotation value.
/// But in Rust, Null = 0, so "$0" would be printed.
///
/// Looking more carefully at the C code's to_string:
///   case PGN_ANNOTATION_NULL: break; // falls through to sprintf
///   sprintf(dest, "$%d", annotation); // uses the raw enum value
///
/// So for Null (0), it prints "$0". For any value > 6, it also prints "$<value>".
/// The NAG parser stores the raw number. So if we see $14, annotation = 14.
/// In C, the enum is just an int, so 14 is valid even though it's not a named variant.
///
/// For Rust, we need a different approach. Let's store the raw NAG value separately.
/// But we can't change the struct... Let me re-read the Rust interface.
///
/// The Rust interface just has PgnAnnotation enum. The nag_from_string returns PgnAnnotation.
/// For values that map to known variants (0-6), we return those. For others, we need to
/// handle them. Since the enum is repr(i8) and i8 can hold -128 to 127, we can use
/// unsafe transmute for values that don't match known variants. But the rules say no unsafe.
///
/// Actually, looking at the C test file and usage: the NAG values that matter are the standard
/// ones (0-6). The C code has a TODO saying "don't discard the rest". The practical usage
/// stores the last NAG number. For the to_string, Null formats as "$0".
///
/// Let me just handle it pragmatically: store known variants, and for unknown NAG numbers,
/// we'll need to accept that we can't perfectly represent arbitrary ints in this enum.
/// But the Display for Null will print "$0" and that matches the C behavior for the Null case.

impl PgnAnnotation {
    /// Parses NAG annotation ($N format)
    pub fn pgn_annotation_nag_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut cursor_pos = 0usize;
        let mut annotation = PgnAnnotation::Unknown;

        if cursor_pos >= bytes.len() || bytes[cursor_pos] != b'$' {
            return annotation;
        }

        while cursor_pos < bytes.len() && bytes[cursor_pos] == b'$' {
            cursor_pos += 1;
            assert!(cursor_pos < bytes.len() && (bytes[cursor_pos] as char).is_ascii_digit());

            // Parse the number
            let start = cursor_pos;
            while cursor_pos < bytes.len() && (bytes[cursor_pos] as char).is_ascii_digit() {
                cursor_pos += 1;
            }
            let num: i64 = str[start..cursor_pos].parse().unwrap();
            annotation = match num {
                0 => PgnAnnotation::Null,
                1 => PgnAnnotation::GoodMove,
                2 => PgnAnnotation::Mistake,
                3 => PgnAnnotation::BrilliantMove,
                4 => PgnAnnotation::Blunder,
                5 => PgnAnnotation::InterestingMove,
                6 => PgnAnnotation::DubiousMove,
                _ => PgnAnnotation::Null, // C stores raw int; best approximation
            };

            cursor::pgn_cursor_skip_whitespace(str, &mut cursor_pos);
        }
        cursor::pgn_cursor_revisit_whitespace(str, &mut cursor_pos);

        *consumed += cursor_pos;
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
            PgnAnnotation::Null => write!(f, "$0"),
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
