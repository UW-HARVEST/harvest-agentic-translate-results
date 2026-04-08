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
enum ScoreSingle {
    Zero = 0,
    One,
    Half,
}

fn score_single_from_string(s: &[u8], consumed: &mut usize) -> Option<ScoreSingle> {
    if s.len() >= 3 && &s[..3] == b"1/2" {
        *consumed += 3;
        Some(ScoreSingle::Half)
    } else if !s.is_empty() && s[0] == b'0' {
        *consumed += 1;
        Some(ScoreSingle::Zero)
    } else if !s.is_empty() && s[0] == b'1' {
        *consumed += 1;
        Some(ScoreSingle::One)
    } else {
        None
    }
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor = 0usize;

        if !bytes.is_empty() && bytes[0] == b'*' {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let white = match score_single_from_string(&bytes[cursor..], &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        if cursor >= bytes.len() || bytes[cursor] != b'-' {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = match score_single_from_string(&bytes[cursor..], &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        *consumed += cursor;

        match (white, black) {
            (ScoreSingle::Half, ScoreSingle::Half) => PgnScore::Draw,
            (ScoreSingle::One, ScoreSingle::Zero) => PgnScore::WhiteWon,
            (ScoreSingle::Zero, ScoreSingle::One) => PgnScore::BlackWon,
            (ScoreSingle::Zero, ScoreSingle::Zero) => PgnScore::Forfeit,
            (ScoreSingle::Zero, ScoreSingle::Half) => PgnScore::WhiteForfeit,
            (ScoreSingle::Half, ScoreSingle::Zero) => PgnScore::BlackForfeit,
            _ => PgnScore::Unknown,
        }
    }

    pub fn from_string_with_consumption_pub(s: &str, consumed: &mut usize) -> Self {
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
        match self {
            PgnScore::Unknown => write!(f, ""),
            PgnScore::Ongoing => write!(f, "*"),
            PgnScore::Draw => write!(f, "1/2-1/2"),
            PgnScore::WhiteWon => write!(f, "1-0"),
            PgnScore::BlackWon => write!(f, "0-1"),
            PgnScore::Forfeit => write!(f, "0-0"),
            PgnScore::WhiteForfeit => write!(f, "0-1/2"),
            PgnScore::BlackForfeit => write!(f, "1/2-0"),
        }
    }
}
