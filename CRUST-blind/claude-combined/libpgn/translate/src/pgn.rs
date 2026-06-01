use crate::utils::cursor::pgn_cursor_skip_whitespace;
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

        let mut moves_consumed = 0usize;
        let moves = PgnMoves::from_string_with_consumption(&s[cursor..], &mut moves_consumed);
        cursor += moves_consumed;
        self.moves = Some(Box::new(moves));

        // Score parsing without exposing the private helper: parse and advance cursor by
        // the canonical string length of the resulting score.
        let score = PgnScore::from(&s[cursor..]);
        let advance = match score {
            PgnScore::Unknown => 0,
            PgnScore::Ongoing => 1,
            PgnScore::Draw => "1/2-1/2".len(),
            PgnScore::WhiteWon => "1-0".len(),
            PgnScore::BlackWon => "0-1".len(),
            PgnScore::Forfeit => "0-0".len(),
            PgnScore::WhiteForfeit => "0-1/2".len(),
            PgnScore::BlackForfeit => "1/2-0".len(),
        };
        cursor += advance;
        self.score = score;

        cursor
    }
}

impl Default for Pgn {
    fn default() -> Self {
        Self::new()
    }
}
