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

impl Default for PgnScore {
    fn default() -> Self {
        PgnScore::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PgnScoreSingle {
    Zero,
    One,
    Half,
}

fn pgn_score_single_from_string(s: &[u8], consumed: &mut usize) -> Option<PgnScoreSingle> {
    if s.len() >= 3 && &s[0..3] == b"1/2" {
        *consumed += 3;
        return Some(PgnScoreSingle::Half);
    }
    if !s.is_empty() {
        if s[0] == b'0' {
            *consumed += 1;
            return Some(PgnScoreSingle::Zero);
        }
        if s[0] == b'1' {
            *consumed += 1;
            return Some(PgnScoreSingle::One);
        }
    }
    None
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        if cursor < bytes.len() && bytes[cursor] == b'*' {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let white = match pgn_score_single_from_string(&bytes[cursor..], &mut cursor) {
            Some(w) => w,
            None => return PgnScore::Unknown,
        };

        if cursor >= bytes.len() || bytes[cursor] != b'-' {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = match pgn_score_single_from_string(&bytes[cursor..], &mut cursor) {
            Some(b) => b,
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
