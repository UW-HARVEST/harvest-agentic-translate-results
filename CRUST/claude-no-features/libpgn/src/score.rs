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
enum PgnScoreSingle {
    Zero,
    One,
    Half,
}

fn pgn_score_single_from_string(s: &str, consumed: &mut usize) -> Option<PgnScoreSingle> {
    let bytes = s.as_bytes();
    if bytes.len() >= 3 && &bytes[..3] == b"1/2" {
        *consumed += 3;
        Some(PgnScoreSingle::Half)
    } else if !bytes.is_empty() && bytes[0] == b'0' {
        *consumed += 1;
        Some(PgnScoreSingle::Zero)
    } else if !bytes.is_empty() && bytes[0] == b'1' {
        *consumed += 1;
        Some(PgnScoreSingle::One)
    } else {
        None
    }
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor = 0usize;

        if !bytes.is_empty() && bytes[0] == b'*' {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let white = match pgn_score_single_from_string(&s[cursor..], &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        if cursor >= bytes.len() || bytes[cursor] != b'-' {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = match pgn_score_single_from_string(&s[cursor..], &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        *consumed += cursor;

        use PgnScoreSingle::*;
        match (white, black) {
            (Half, Half) => PgnScore::Draw,
            (One, Zero) => PgnScore::WhiteWon,
            (Zero, One) => PgnScore::BlackWon,
            (Zero, Zero) => PgnScore::Forfeit,
            (Zero, Half) => PgnScore::WhiteForfeit,
            (Half, Zero) => PgnScore::BlackForfeit,
            _ => PgnScore::Unknown,
        }
    }
}

impl From<&str> for PgnScore {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
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
        write!(f, "{}", s)
    }
}
