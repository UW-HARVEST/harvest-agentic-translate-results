#[repr(i8)] // Ensures binary compatibility with C enums
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgnCheck {
    Mate = -1,
    None = 0,
    Single,
    Double,
}

impl PgnCheck {
    fn from_i32(v: i32) -> Self {
        match v {
            -1 => PgnCheck::Mate,
            0 => PgnCheck::None,
            1 => PgnCheck::Single,
            2 => PgnCheck::Double,
            _ => PgnCheck::None,
        }
    }

    /// Parses a PGN check annotation from a string, tracking characters consumed.
    pub fn __pgn_check_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut cursor = 0usize;
        let mut check_count = 0i32;
        let mut check = PgnCheck::None;

        if cursor < bytes.len() {
            match bytes[cursor] {
                b'+' => {
                    while cursor < bytes.len() && bytes[cursor] == b'+' {
                        check_count += 1;
                        cursor += 1;
                    }
                    debug_assert!(check_count <= 2);
                    check = PgnCheck::from_i32(check_count);
                }
                b'#' => {
                    check = PgnCheck::Mate;
                    cursor += 1;
                }
                _ => {}
            }
        }

        *consumed += cursor;
        check
    }
}

impl From<&str> for PgnCheck {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnCheck::__pgn_check_from_string(s, &mut consumed)
    }
}
