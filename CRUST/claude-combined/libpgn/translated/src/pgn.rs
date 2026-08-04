use crate::{
    metadata::PgnMetadata,
    moves::{PgnMove, PgnMoves},
    score::PgnScore,
    utils::cursor::pgn_cursor_skip_whitespace,
};
#[derive(Debug)]
pub struct Pgn {
    pub metadata: Option<Box<PgnMetadata>>, // Instead of raw `pgn_metadata_t *`
    pub moves: Option<Box<PgnMoves>>,       // Instead of raw `pgn_moves_t *`
    pub score: PgnScore,
}
impl Pgn {
    pub fn new() -> Self {
        Pgn {
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

        let metadata = PgnMetadata::from_string_with_consumption(s, &mut cursor);
        self.metadata = Some(Box::new(metadata));

        pgn_cursor_skip_whitespace(s, &mut cursor);

        let moves = PgnMoves::from_string_with_consumption(&s[cursor..], &mut cursor);
        self.moves = Some(Box::new(moves));

        pgn_cursor_skip_whitespace(s, &mut cursor);
        self.score = PgnScore::from(&s[cursor..]);
        // Compute consumed for score
        let mut score_cursor = 0usize;
        let _ = crate::score::pgn_score_from_string_with_consumption(&s[cursor..], &mut score_cursor);
        cursor += score_cursor;

        cursor
    }
}

impl Default for Pgn {
    fn default() -> Self {
        Self::new()
    }
}
