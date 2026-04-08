use crate::{
    metadata::PgnMetadata,
    moves::{PgnMove, PgnMoves},
    score::PgnScore,
    utils::cursor,
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
        let mut cur = 0usize;

        let metadata = PgnMetadata::from_string_with_consumption(&s[cur..], &mut cur);
        // Only store if it has items (C returns NULL if no '[')
        if !s.is_empty() && s.as_bytes()[0] == b'[' {
            self.metadata = Some(Box::new(metadata));
        }

        cursor::pgn_cursor_skip_whitespace(s, &mut cur);

        let moves = PgnMoves::from_string_with_consumption(&s[cur..], &mut cur);
        self.moves = Some(Box::new(moves));

        let mut score_consumed = 0usize;
        self.score = PgnScore::from_string_with_consumption_pub(&s[cur..], &mut score_consumed);
        cur += score_consumed;

        cur
    }
}
