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

fn parse_single_score(s: &str, consumed: &mut usize) -> Option<u8> {
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

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let mut cursor = 0usize;

        if s.as_bytes().first() == Some(&b'*') {
            *consumed += 1;
            return Self::Ongoing;
        }

        let Some(white) = parse_single_score(&s[cursor..], &mut cursor) else {
            return Self::Unknown;
        };

        if s.as_bytes().get(cursor) != Some(&b'-') {
            return Self::Unknown;
        }
        cursor += 1;

        let Some(black) = parse_single_score(&s[cursor..], &mut cursor) else {
            return Self::Unknown;
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
        let text = match self {
            Self::Unknown => "",
            Self::Ongoing => "*",
            Self::Draw => "1/2-1/2",
            Self::WhiteWon => "1-0",
            Self::BlackWon => "0-1",
            Self::Forfeit => "0-0",
            Self::WhiteForfeit => "0-1/2",
            Self::BlackForfeit => "1/2-0",
        };
        f.write_str(text)
    }
}
