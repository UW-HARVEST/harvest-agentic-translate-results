#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PgnScore {
    Unknown = 0,
    Ongoing,
    Draw,
    WhiteWon,
    BlackWon,
    Forfeit,
    WhiteForfeit,
    BlackForfeit,
}
impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        fn parse_single(s: &str, consumed: &mut usize) -> Option<u8> {
            if s.starts_with("1/2") {
                *consumed += 3;
                Some(2)
            } else if s.starts_with('0') {
                *consumed += 1;
                Some(0)
            } else if s.starts_with('1') {
                *consumed += 1;
                Some(1)
            } else {
                None
            }
        }

        let mut cursor = 0;
        if s.starts_with('*') {
            *consumed += 1;
            return Self::Ongoing;
        }

        let white = match parse_single(&s[cursor..], &mut cursor) {
            Some(value) => value,
            None => return Self::Unknown,
        };

        if !matches!(s.as_bytes().get(cursor), Some(b'-')) {
            return Self::Unknown;
        }
        cursor += 1;

        let black = match parse_single(&s[cursor..], &mut cursor) {
            Some(value) => value,
            None => return Self::Unknown,
        };

        *consumed += cursor;

        match (white, black) {
            (2, 2) => Self::Draw,
            (1, 0) => Self::WhiteWon,
            (0, 1) => Self::BlackWon,
            (0, 0) => Self::Forfeit,
            (0, 2) => Self::WhiteForfeit,
            (2, 0) => Self::BlackForfeit,
            _ => Self::Unknown,
        }
    }
}
impl From<&str> for PgnScore {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        Self::from_string_with_consumption(s, &mut consumed)
    }
}
impl std::fmt::Display for PgnScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Unknown => "",
            Self::Ongoing => "*",
            Self::Draw => "1/2-1/2",
            Self::WhiteWon => "1-0",
            Self::BlackWon => "0-1",
            Self::Forfeit => "0-0",
            Self::WhiteForfeit => "0-1/2",
            Self::BlackForfeit => "1/2-0",
        };
        f.write_str(s)
    }
}
