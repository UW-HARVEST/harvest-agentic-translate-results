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

        let metadata = PgnMetadata::from_string_with_consumption(&s[cursor..], &mut cursor);
        self.metadata = Some(Box::new(metadata));
        pgn_cursor_skip_whitespace(s, &mut cursor);

        let moves = PgnMoves::from_string_with_consumption(&s[cursor..], &mut cursor);
        self.moves = Some(Box::new(moves));

        self.score = PgnScore::from_string_with_consumption(&s[cursor..], &mut cursor);

        cursor
    }
}

impl Default for Pgn {
    fn default() -> Self {
        Self::new()
    }
}
