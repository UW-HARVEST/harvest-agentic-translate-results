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
    Invalid,
}

fn pgn_score_single_from_string(s: &str, cursor: &mut usize, consumed: &mut usize) -> PgnScoreSingle {
    let bytes = s.as_bytes();
    if *cursor + 3 <= bytes.len() && &bytes[*cursor..*cursor + 3] == b"1/2" {
        *cursor += 3;
        *consumed += 3;
        return PgnScoreSingle::Half;
    }
    if *cursor < bytes.len() {
        if bytes[*cursor] == b'0' {
            *cursor += 1;
            *consumed += 1;
            return PgnScoreSingle::Zero;
        } else if bytes[*cursor] == b'1' {
            *cursor += 1;
            *consumed += 1;
            return PgnScoreSingle::One;
        }
    }
    PgnScoreSingle::Invalid
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        if !bytes.is_empty() && bytes[cursor] == b'*' {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let mut local_consumed: usize = 0;
        let white = pgn_score_single_from_string(s, &mut cursor, &mut local_consumed);
        if white == PgnScoreSingle::Invalid {
            return PgnScore::Unknown;
        }

        if cursor >= bytes.len() || bytes[cursor] != b'-' {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = pgn_score_single_from_string(s, &mut cursor, &mut local_consumed);
        if black == PgnScoreSingle::Invalid {
            return PgnScore::Unknown;
        }

        *consumed += cursor;

        match (white, black) {
            (PgnScoreSingle::Half, PgnScoreSingle::Half) => PgnScore::Draw,
            (PgnScoreSingle::One, PgnScoreSingle::Zero) => PgnScore::WhiteWon,
            (PgnScoreSingle::Zero, PgnScoreSingle::One) => PgnScore::BlackWon,
            (PgnScoreSingle::Zero, PgnScoreSingle::Zero) => PgnScore::Forfeit,
            (PgnScoreSingle::Zero, PgnScoreSingle::Half) => PgnScore::WhiteForfeit,
            (PgnScoreSingle::Half, PgnScoreSingle::Zero) => PgnScore::BlackForfeit,
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
        write!(f, "{}", s)
    }
}
