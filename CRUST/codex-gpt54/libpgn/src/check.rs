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
        let mut cursor = 0usize;
        let mut check = Self::None;

        match bytes.get(cursor).copied() {
            Some(b'+') => {
                while bytes.get(cursor) == Some(&b'+') {
                    check = match check {
                        Self::None => Self::Single,
                        Self::Single => Self::Double,
                        Self::Double => Self::Double,
                        Self::Mate => Self::Mate,
                    };
                    cursor += 1;
                }
                assert!(matches!(check, Self::Single | Self::Double));
            }
            Some(b'#') => {
                check = Self::Mate;
                cursor += 1;
            }
            _ => {}
        }

        *consumed += cursor;
        check
    }
}
impl From<&str> for PgnCheck {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        Self::__pgn_check_from_string(s, &mut consumed)
    }
}
