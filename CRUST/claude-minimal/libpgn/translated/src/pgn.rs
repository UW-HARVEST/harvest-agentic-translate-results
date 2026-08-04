use crate::{
    metadata::PgnMetadata,
    moves::{PgnMove, PgnMoves},
    score::PgnScore,
    utils::cursor::pgn_cursor_skip_whitespace,
};
#[derive(Debug)]
pub struct Pgn {
    pub metadata: Option<Box<PgnMetadata>>,
    pub moves: Option<Box<PgnMoves>>,
    pub score: PgnScore,
}
impl Default for Pgn {
    fn default() -> Self {
        Self::new()
    }
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
        let mut cursor: usize = 0;

        let mut consumed = 0;
        let metadata = PgnMetadata::from_string_with_consumption(s, &mut consumed);
        cursor += consumed;
        self.metadata = Some(Box::new(metadata));

        pgn_cursor_skip_whitespace(s, &mut cursor);

        let mut consumed = 0;
        let moves = PgnMoves::from_string_with_consumption(&s[cursor..], &mut consumed);
        cursor += consumed;
        self.moves = Some(Box::new(moves));

        let mut consumed = 0;
        self.score = PgnScore::from_string_with_consumption(&s[cursor..], &mut consumed);
        cursor += consumed;

        cursor
    }
}
