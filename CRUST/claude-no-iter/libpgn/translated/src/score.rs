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

fn pgn_score_single_from_string(s: &[u8], cursor: &mut usize) -> Option<PgnScoreSingle> {
    if cursor.checked_add(3).map(|c| c <= s.len()).unwrap_or(false) && &s[*cursor..*cursor + 3] == b"1/2" {
        *cursor += 3;
        return Some(PgnScoreSingle::Half);
    }
    if *cursor < s.len() {
        match s[*cursor] {
            b'0' => {
                *cursor += 1;
                return Some(PgnScoreSingle::Zero);
            }
            b'1' => {
                *cursor += 1;
                return Some(PgnScoreSingle::One);
            }
            _ => {}
        }
    }
    None
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    pub(crate) fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        if !bytes.is_empty() && bytes[0] == b'*' {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let white = match pgn_score_single_from_string(bytes, &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

        if cursor >= bytes.len() || bytes[cursor] != b'-' {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = match pgn_score_single_from_string(bytes, &mut cursor) {
            Some(v) => v,
            None => return PgnScore::Unknown,
        };

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
        f.write_str(s)
    }
}
