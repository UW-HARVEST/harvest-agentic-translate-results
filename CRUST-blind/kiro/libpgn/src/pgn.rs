use crate::{
    metadata::PgnMetadata,
    moves::{PgnMove, PgnMoves},
    score::PgnScore,
};
use crate::utils::cursor::pgn_cursor_skip_whitespace;

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
        // C code returns NULL if no '[' found; our version returns empty metadata
        // Check if metadata has items
        if s.as_bytes().first() == Some(&b'[') {
            self.metadata = Some(Box::new(metadata));
        }

        pgn_cursor_skip_whitespace(s, &mut cursor);

        let mut moves_consumed = 0usize;
        let moves = PgnMoves::from_string_with_consumption(&s[cursor..], &mut moves_consumed);
        cursor += moves_consumed;
        self.moves = Some(Box::new(moves));

        let mut score_consumed = 0usize;
        self.score = PgnScore::from_string_with_consumption_pub(&s[cursor..], &mut score_consumed);
        cursor += score_consumed;

        cursor
    }
}
