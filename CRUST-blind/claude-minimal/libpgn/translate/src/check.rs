#[repr(i8)] // Ensures binary compatibility with C enums
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgnCheck {
    Mate = -1,
    None = 0,
    Single,
    Double,
}
impl PgnCheck {
    /// Parses a PGN check annotation from a string, tracking characters consumed.
    pub fn __pgn_check_from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut cursor: usize = 0;
        let mut check = PgnCheck::None;

        if !bytes.is_empty() {
            match bytes[cursor] {
                b'+' => {
                    while cursor < bytes.len() && bytes[cursor] == b'+' {
                        check = match check {
                            PgnCheck::None => PgnCheck::Single,
                            PgnCheck::Single => PgnCheck::Double,
                            _ => panic!("too many +"),
                        };
                        cursor += 1;
                    }
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
        let mut consumed = 0;
        PgnCheck::__pgn_check_from_string(s, &mut consumed)
    }
}
