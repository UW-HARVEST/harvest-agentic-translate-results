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
    Invalid,
}

fn parse_single(s: &str, consumed: &mut usize) -> SingleScore {
    let bytes = s.as_bytes();
    if bytes.len() >= 3 && &bytes[..3] == b"1/2" {
        *consumed += 3;
        return SingleScore::Half;
    }
    if !bytes.is_empty() {
        match bytes[0] {
            b'0' => {
                *consumed += 1;
                return SingleScore::Zero;
            }
            b'1' => {
                *consumed += 1;
                return SingleScore::One;
            }
            _ => {}
        }
    }
    SingleScore::Invalid
}

impl PgnScore {
    /// Parses a PGN score from a string, tracking characters consumed (`__pgn_score_from_string`)
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        if !bytes.is_empty() && bytes[0] == b'*' {
            cursor += 1;
            *consumed += cursor;
            return PgnScore::Ongoing;
        }

        let white = parse_single(&s[cursor..], &mut cursor);
        if white == SingleScore::Invalid {
            return PgnScore::Unknown;
        }

        if cursor >= bytes.len() || bytes[cursor] != b'-' {
            return PgnScore::Unknown;
        }
        cursor += 1;

        let black = parse_single(&s[cursor..], &mut cursor);
        if black == SingleScore::Invalid {
            return PgnScore::Unknown;
        }

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
}

impl From<&str> for PgnScore {
    fn from(s: &str) -> Self {
        let mut consumed: usize = 0;
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
