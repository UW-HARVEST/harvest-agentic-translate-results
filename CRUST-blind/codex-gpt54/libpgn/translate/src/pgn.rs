use crate::{
    metadata::PgnMetadata,
    moves::{PgnMove, PgnMoves},
    score::PgnScore,
};
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
        let mut cursor = 0;

        if s.starts_with('[') {
            let metadata = PgnMetadata::from_string_with_consumption(&s[cursor..], &mut cursor);
            self.metadata = Some(Box::new(metadata));
        } else {
            self.metadata = None;
        }

        while matches!(s.as_bytes().get(cursor), Some(b) if (*b as char).is_ascii_whitespace()) {
            cursor += 1;
        }

        let moves = PgnMoves::from_string_with_consumption(&s[cursor..], &mut cursor);
        self.moves = Some(Box::new(moves));
        let score = PgnScore::from(&s[cursor..]);
        let score_len = score.to_string().len();
        self.score = score;
        cursor += score_len;

        cursor
    }
}
