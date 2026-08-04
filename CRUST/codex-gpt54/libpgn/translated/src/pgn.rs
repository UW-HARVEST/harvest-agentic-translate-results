use crate::{
    metadata::PgnMetadata,
    moves::{PgnMove, PgnMoves},
    score::PgnScore,
};

fn skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

fn score_len(score: PgnScore) -> usize {
    match score {
        PgnScore::Unknown => 0,
        PgnScore::Ongoing => 1,
        PgnScore::Draw => 7,
        PgnScore::WhiteWon | PgnScore::BlackWon | PgnScore::Forfeit => 3,
        PgnScore::WhiteForfeit | PgnScore::BlackForfeit => 5,
    }
}

#[derive(Debug)]
pub struct Pgn {
    pub metadata: Option<Box<PgnMetadata>>, // Instead of raw `pgn_metadata_t *`
    pub moves: Option<Box<PgnMoves>>,       // Instead of raw `pgn_moves_t *`
    pub score: PgnScore,
}
impl Pgn {
    pub fn new() -> Self {
        Self {
            metadata: None,
            moves: None,
            score: PgnScore::Unknown,
        }
    }
    pub fn parse_metadata(s: &str) -> PgnMetadata {
        PgnMetadata::from_string(s)
    }
    pub fn parse_move(s: &str) -> PgnMove {
        PgnMove::from(s)
    }
    pub fn parse_moves(s: &str) -> PgnMoves {
        PgnMoves::from(s)
    }
    pub fn parse_score(s: &str) -> PgnScore {
        PgnScore::from(s)
    }
    pub fn parse(&mut self, s: &str) -> usize {
        let mut cursor = 0usize;

        let mut metadata_consumed = 0usize;
        let metadata = PgnMetadata::from_string_with_consumption(&s[cursor..], &mut metadata_consumed);
        if metadata_consumed > 0 {
            self.metadata = Some(Box::new(metadata));
            cursor += metadata_consumed;
        } else {
            self.metadata = None;
        }

        skip_whitespace(s, &mut cursor);

        let mut moves_consumed = 0usize;
        self.moves = Some(Box::new(PgnMoves::from_string_with_consumption(
            &s[cursor..],
            &mut moves_consumed,
        )));
        cursor += moves_consumed;

        self.score = PgnScore::from(&s[cursor..]);
        cursor += score_len(self.score);

        cursor
    }
}
