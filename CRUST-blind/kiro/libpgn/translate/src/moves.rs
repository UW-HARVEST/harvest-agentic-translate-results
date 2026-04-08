use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::PgnComments, coordinate::PgnCoordinate,
    piece::PgnPiece,
};
use crate::comments::PgnCommentPosition;
use crate::score::PgnScore;
use crate::utils::cursor::{pgn_cursor_skip_whitespace, pgn_cursor_revisit_whitespace};

pub const PGN_CASTLING_NONE: u8 = 0;
pub const PGN_CASTLING_KINGSIDE: u8 = 2;
pub const PGN_CASTLING_QUEENSIDE: u8 = 3;
pub const PGN_MOVES_INITIAL_SIZE: usize = 32;
pub const PGN_MOVES_GROW_SIZE: usize = 32;
pub const PGN_MOVE_NOTATION_SIZE: usize = 16;
pub const PGN_ALTERNATIVE_MOVES_INITIAL_SIZE: usize = 1;
pub const PGN_ALTERNATIVE_MOVES_GROW_SIZE: usize = 1;

const PGN_EXPECT_WHITE: i32 = 0;
const PGN_EXPECT_BLACK: i32 = 1;

#[derive(Debug)]
pub struct PgnMove {
    pub piece: PgnPiece,
    pub promoted_to: PgnPiece,
    pub notation: String,
    pub castles: u8,
    pub captures: bool,
    pub en_passant: bool,
    pub check: PgnCheck,
    pub from: PgnCoordinate,
    pub dest: PgnCoordinate,
    pub annotation: PgnAnnotation,
    pub comments: Option<PgnComments>,
    pub alternatives: Option<PgnAlternativeMoves>,
}
impl Default for PgnMove {
    fn default() -> Self {
        PgnMove {
            piece: PgnPiece::Unknown,
            promoted_to: PgnPiece::Unknown,
            notation: String::new(),
            castles: PGN_CASTLING_NONE,
            captures: false,
            en_passant: false,
            check: PgnCheck::None,
            from: PgnCoordinate { file: None, rank: None },
            dest: PgnCoordinate { file: None, rank: None },
            annotation: PgnAnnotation::Unknown,
            comments: None,
            alternatives: None,
        }
    }
}
impl PgnMove {
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut move_ = PgnMove::default();
        let mut cursor = 0usize;

        // Castling
        if cursor < bytes.len() && bytes[cursor] == b'O' {
            cursor += 1;
            assert!(bytes[cursor] == b'-'); cursor += 1;
            assert!(bytes[cursor] == b'O'); cursor += 1;
            move_.castles = PGN_CASTLING_KINGSIDE;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert!(bytes[cursor] == b'O'); cursor += 1;
                move_.castles = PGN_CASTLING_QUEENSIDE;
            }

            // goto check
            move_.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
            move_.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

            let skipped_ws_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);
            // en passant check (unlikely for castling but following C logic)
            if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
                assert!(skipped_ws_before_ep);
                assert!(bytes[cursor] == b'e');
                cursor += 1; assert!(bytes[cursor] == b'.'); 
                cursor += 1; assert!(bytes[cursor] == b'p');
                cursor += 1; assert!(bytes[cursor] == b'.');
                cursor += 1;
                move_.en_passant = true;
            }

            let skipped_ws_after_ep = if move_.en_passant { pgn_cursor_skip_whitespace(s, &mut cursor) } else { skipped_ws_before_ep };

            if move_.annotation == PgnAnnotation::Unknown {
                move_.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
                if move_.annotation != PgnAnnotation::Unknown {
                    assert!(skipped_ws_after_ep);
                }
            }

            pgn_cursor_revisit_whitespace(s, &mut cursor);
            move_.notation = s[..cursor].to_string();
            *consumed += cursor;
            return move_;
        }

        // Piece
        move_.piece = PgnPiece::from(bytes[cursor] as char);
        cursor += 1;
        if move_.piece == PgnPiece::Unknown {
            move_.piece = PgnPiece::Pawn;
            cursor -= 1;
        }

        // From coordinate
        if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() && bytes[cursor] != b'x' {
            move_.from.file = Some(bytes[cursor] as char);
            cursor += 1;
        }
        if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            move_.from.rank = Some((bytes[cursor] - b'0') as i32);
            cursor += 1;
        }

        // Captures
        move_.captures = cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':');
        if move_.captures { cursor += 1; }

        // Destination coordinate
        if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
            move_.dest.file = Some(bytes[cursor] as char);
            cursor += 1;
            assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
            move_.dest.rank = Some((bytes[cursor] - b'0') as i32);
            cursor += 1;
        } else {
            move_.dest = move_.from;
            move_.from = PgnCoordinate { file: None, rank: None };
        }

        // Promotion
        move_.promoted_to = if cursor < bytes.len() {
            PgnPiece::from(bytes[cursor] as char)
        } else {
            PgnPiece::Unknown
        };
        if move_.promoted_to == PgnPiece::Unknown {
            if cursor < bytes.len() {
                match bytes[cursor] {
                    b'(' => {
                        cursor += 1;
                        move_.promoted_to = PgnPiece::from(bytes[cursor] as char);
                        assert!(move_.promoted_to != PgnPiece::Unknown);
                        cursor += 1;
                        assert!(bytes[cursor] == b')');
                        cursor += 1;
                    }
                    b'=' | b'/' => {
                        cursor += 1;
                        move_.promoted_to = PgnPiece::from(bytes[cursor] as char);
                        assert!(move_.promoted_to != PgnPiece::Unknown);
                        cursor += 1;
                    }
                    _ => {}
                }
            }
        } else {
            cursor += 1;
        }

        assert!(move_.dest.file.is_some());
        assert!(move_.dest.rank.is_some());

        // Check
        move_.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        move_.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_ws_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // En passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            assert!(skipped_ws_before_ep);
            assert!(bytes[cursor] == b'e');
            cursor += 1; assert!(bytes[cursor] == b'.');
            cursor += 1; assert!(bytes[cursor] == b'p');
            cursor += 1; assert!(bytes[cursor] == b'.');
            cursor += 1;
            move_.en_passant = true;
        }

        let skipped_ws_after_ep = if move_.en_passant { pgn_cursor_skip_whitespace(s, &mut cursor) } else { skipped_ws_before_ep };

        // NAG annotation
        if move_.annotation == PgnAnnotation::Unknown {
            move_.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if move_.annotation != PgnAnnotation::Unknown {
                assert!(skipped_ws_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);
        move_.notation = s[..cursor].to_string();
        *consumed += cursor;
        move_
    }
}
impl From<&str> for PgnMove {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        PgnMove::from_string_with_consumption(s, &mut consumed)
    }
}
impl Display for PgnMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();

        match self.castles {
            PGN_CASTLING_KINGSIDE => {
                result.push_str("O-O");
                // goto check
                Self::append_check_and_annotation(&mut result, self);
                return write!(f, "{}", result);
            }
            PGN_CASTLING_QUEENSIDE => {
                result.push_str("O-O-O");
                Self::append_check_and_annotation(&mut result, self);
                return write!(f, "{}", result);
            }
            _ => {}
        }

        assert!(self.piece != PgnPiece::Unknown);
        if self.piece != PgnPiece::Pawn {
            result.push(self.piece as u8 as char);
        }

        if let Some(file) = self.from.file { result.push(file); }
        if let Some(rank) = self.from.rank { result.push((b'0' + rank as u8) as char); }

        if self.captures { result.push('x'); }

        if let Some(file) = self.dest.file { result.push(file); }
        if let Some(rank) = self.dest.rank { result.push((b'0' + rank as u8) as char); }

        if self.promoted_to != PgnPiece::Unknown {
            assert!(self.piece == PgnPiece::Pawn);
            assert!(self.promoted_to != PgnPiece::Pawn);
            result.push('=');
            result.push(self.promoted_to as u8 as char);
        }

        Self::append_check_and_annotation(&mut result, self);
        write!(f, "{}", result)
    }
}

impl PgnMove {
    fn append_check_and_annotation(result: &mut String, move_: &PgnMove) {
        match move_.check {
            PgnCheck::None => {}
            PgnCheck::Mate => result.push('#'),
            PgnCheck::Single => result.push('+'),
            PgnCheck::Double => { result.push('+'); result.push('+'); }
        }

        // Standard annotations (GoodMove..DubiousMove)
        let ann_val = move_.annotation as i8;
        if ann_val >= PgnAnnotation::GoodMove as i8 && ann_val <= PgnAnnotation::DubiousMove as i8 {
            result.push_str(&move_.annotation.to_string());
        }

        if move_.en_passant {
            result.push(' ');
            result.push_str("e.p.");
        }

        // NAG or Null annotation
        if ann_val > PgnAnnotation::DubiousMove as i8 || ann_val == PgnAnnotation::Null as i8 {
            result.push(' ');
            result.push_str(&move_.annotation.to_string());
        }
    }
}

#[derive(Debug)]
pub struct PgnMoves {
    pub values: Vec<PgnMovesItem>,
}
impl From<&str> for PgnMoves {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        PgnMoves::from_string_with_consumption(s, &mut consumed)
    }
}
impl PgnMoves {
    pub fn new() -> Self {
        PgnMoves { values: Vec::new() }
    }
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let moves = PgnMoves::new();
        pgn_moves_from_string_recurse(s, consumed, moves, PGN_EXPECT_WHITE)
    }
    pub fn push(&mut self, moves: PgnMovesItem) {
        self.values.push(moves);
    }
}
#[derive(Debug)]
pub struct PgnMovesItem {
    pub white: PgnMove,
    pub black: PgnMove,
}
#[derive(Debug)]
pub struct PgnAlternativeMoves {
    pub values: Vec<Box<PgnMoves>>,
}
impl PgnAlternativeMoves {
    pub fn new() -> Self {
        PgnAlternativeMoves { values: Vec::new() }
    }
    pub fn poll(
        alt: &mut Option<Self>,
        placeholder: &mut Option<PgnComments>,
        s: &str,
        expect: i32,
    ) -> usize {
        let bytes = s.as_bytes();
        let mut cursor = 0usize;

        while cursor < bytes.len() && bytes[cursor] == b'(' {
            cursor += 1;

            if alt.is_none() {
                *alt = Some(PgnAlternativeMoves::new());
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);
            let mut inner_consumed = 0usize;
            let inner_moves = pgn_moves_from_string_recurse(&s[cursor..], &mut inner_consumed, PgnMoves::new(), expect);
            cursor += inner_consumed;
            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            alt.as_mut().unwrap().push(inner_moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);

            // Poll comments with AfterAlternative position
            if cursor < bytes.len() && bytes[cursor] == b'{' {
                if placeholder.is_none() {
                    *placeholder = Some(PgnComments::new());
                }
                let c = placeholder.as_mut().unwrap().poll(PgnCommentPosition::AfterAlternative, &s[cursor..]);
                cursor += c;
            }
        }

        cursor
    }
    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }
}

fn comments_poll_helper(comments: &mut Option<PgnComments>, pos: PgnCommentPosition, s: &str) -> usize {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return 0;
    }
    if comments.is_none() {
        *comments = Some(PgnComments::new());
    }
    comments.as_mut().unwrap().poll(pos, s)
}

fn pgn_moves_from_string_recurse(s: &str, consumed: &mut usize, mut moves: PgnMoves, expect: i32) -> PgnMoves {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' {
        return moves;
    }

    let mut cursor = 0usize;
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() { cursor += 1; }

    let mut dots_count = 0;
    assert!(cursor < bytes.len() && bytes[cursor] == b'.');
    while cursor < bytes.len() && bytes[cursor] == b'.' {
        cursor += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE { assert!(dots_count == 1); }
    if expect == PGN_EXPECT_BLACK { assert!(dots_count == 3); }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        // Black move only
        let mut black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += comments_poll_helper(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
        cursor += PgnAlternativeMoves::poll(&mut black.alternatives, &mut comments, &s[cursor..], PGN_EXPECT_BLACK);
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += comments_poll_helper(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

        if let Some(c) = comments.take() {
            black.comments = Some(c);
        }

        moves.push(PgnMovesItem { white: PgnMove::default(), black });
        let mut inner_consumed = 0usize;
        moves = pgn_moves_from_string_recurse(&s[cursor..], &mut inner_consumed, moves, PGN_EXPECT_WHITE);
        cursor += inner_consumed;
        *consumed += cursor;
        return moves;
    }

    // White move
    let mut white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(&mut white.alternatives, &mut comments, &s[cursor..], PGN_EXPECT_WHITE);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if let Some(c) = comments.take() {
        white.comments = Some(c);
    }

    // Check for score (end of game)
    if PgnScore::from(&s[cursor..]) != PgnScore::Unknown {
        moves.push(PgnMovesItem { white, black: PgnMove::default() });
        *consumed += cursor;
        return moves;
    }

    // End of string or end of alternative
    if cursor >= bytes.len() || bytes[cursor] == b')' {
        moves.push(PgnMovesItem { white, black: PgnMove::default() });
        *consumed += cursor;
        return moves;
    }

    // Black move
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    // Optional move number for black (e.g., "1...")
    if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() { cursor += 1; }
        for _ in 0..3 {
            assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
        }
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    let mut black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(&mut black.alternatives, &mut comments, &s[cursor..], PGN_EXPECT_BLACK);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += comments_poll_helper(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if let Some(c) = comments.take() {
        black.comments = Some(c);
    }

    moves.push(PgnMovesItem { white, black });

    if PgnScore::from(&s[cursor..]) != PgnScore::Unknown {
        *consumed += cursor;
        return moves;
    }

    let mut inner_consumed = 0usize;
    moves = pgn_moves_from_string_recurse(&s[cursor..], &mut inner_consumed, moves, PGN_EXPECT_WHITE);
    cursor += inner_consumed;
    *consumed += cursor;
    moves
}
