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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SingleScore {
    Zero,
    One,
    Half,
}

fn parse_single(s: &str, consumed: &mut usize) -> Option<SingleScore> {
    let bytes = s.as_bytes();
    if s.starts_with("1/2") {
        *consumed += 3;
        return Some(SingleScore::Half);
    }
    match bytes.first().copied() {
        Some(b'0') => {
            *consumed += 1;
            Some(SingleScore::Zero)
        }
        Some(b'1') => {
            *consumed += 1;
            Some(SingleScore::One)
        }
        _ => None,
    }
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        if bytes.first().copied() == Some(b'*') {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let white = match parse_single(&s[cursor..], &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        if bytes.get(cursor).copied() != Some(b'-') {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = match parse_single(&s[cursor..], &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        *consumed += cursor;

        match (white, black) {
            (SingleScore::Half, SingleScore::Half) => PgnScore::Draw,
            (SingleScore::One, SingleScore::Zero) => PgnScore::WhiteWon,
            (SingleScore::Zero, SingleScore::One) => PgnScore::BlackWon,
            (SingleScore::Zero, SingleScore::Zero) => PgnScore::Forfeit,
            (SingleScore::Zero, SingleScore::Half) => PgnScore::WhiteForfeit,
            (SingleScore::Half, SingleScore::Zero) => PgnScore::BlackForfeit,
            _ => PgnScore::Unknown,
        }
    }

    /// Parses a PGN score from a string with a mutable consumed counter.
    pub fn parse_with_consumed(s: &str, consumed: &mut usize) -> Self {
        Self::from_string_with_consumption(s, consumed)
    }
}

impl From<&str> for PgnScore {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        PgnScore::from_string_with_consumption(s, &mut consumed)
    }
}

impl std::fmt::Display for PgnScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PgnScore::Unknown => "",
            PgnScore::Ongoing => "*",
            PgnScore::Draw => "1/2-1/2",
            PgnScore::WhiteWon => "1-0",
            PgnScore::BlackWon => "0-1",
            PgnScore::Forfeit => "0-0",
            PgnScore::WhiteForfeit => "0-1/2",
            PgnScore::BlackForfeit => "1/2-0",
        };
        f.write_str(s)
    }
}
