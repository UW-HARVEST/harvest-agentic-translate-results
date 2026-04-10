use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::{PgnComments, PgnCommentPosition},
    coordinate::PgnCoordinate, piece::PgnPiece, score::PgnScore,
    utils::cursor,
};
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
        let mut cur = 0usize;
        let mut mv = PgnMove::default();

        // Castling
        if cur < bytes.len() && bytes[cur] == b'O' {
            cur += 1;
            assert!(cur < bytes.len() && bytes[cur] == b'-');
            cur += 1;
            assert!(cur < bytes.len() && bytes[cur] == b'O');
            cur += 1;
            mv.castles = PGN_CASTLING_KINGSIDE;

            if cur < bytes.len() && bytes[cur] == b'-' {
                cur += 1;
                assert!(cur < bytes.len() && bytes[cur] == b'O');
                cur += 1;
                mv.castles = PGN_CASTLING_QUEENSIDE;
            }

            // goto check
            mv.check = PgnCheck::__pgn_check_from_string(&s[cur..], &mut cur);
            mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cur..], &mut cur);
            parse_ep_and_nag(s, &mut cur, &mut mv);
            cursor::pgn_cursor_revisit_whitespace(s, &mut cur);
            mv.notation = s[..cur].to_string();
            *consumed += cur;
            return mv;
        }

        // Piece
        if cur < bytes.len() {
            mv.piece = PgnPiece::from(bytes[cur] as char);
            if mv.piece != PgnPiece::Unknown {
                cur += 1;
            } else {
                mv.piece = PgnPiece::Pawn;
            }
        }

        // From coordinate (file)
        if cur < bytes.len() && (bytes[cur] as char).is_ascii_lowercase() && bytes[cur] != b'x' {
            mv.from.file = Some(bytes[cur] as char);
            cur += 1;
        }
        // From coordinate (rank)
        if cur < bytes.len() && (bytes[cur] as char).is_ascii_digit() {
            mv.from.rank = Some((bytes[cur] - b'0') as i32);
            cur += 1;
        }

        // Captures
        if cur < bytes.len() && (bytes[cur] == b'x' || bytes[cur] == b':') {
            mv.captures = true;
            cur += 1;
        }

        // Destination
        if cur < bytes.len() && (bytes[cur] as char).is_ascii_lowercase() {
            mv.dest.file = Some(bytes[cur] as char);
            cur += 1;
            assert!(cur < bytes.len() && (bytes[cur] as char).is_ascii_digit());
            mv.dest.rank = Some((bytes[cur] - b'0') as i32);
            cur += 1;
        } else {
            mv.dest = mv.from;
            mv.from = PgnCoordinate { file: None, rank: None };
        }

        // Promotion
        if cur < bytes.len() {
            let promoted = PgnPiece::from(bytes[cur] as char);
            if promoted != PgnPiece::Unknown {
                mv.promoted_to = promoted;
                cur += 1;
            } else {
                match bytes[cur] {
                    b'(' => {
                        cur += 1;
                        mv.promoted_to = PgnPiece::from(bytes[cur] as char);
                        assert!(mv.promoted_to != PgnPiece::Unknown);
                        cur += 1;
                        assert!(cur < bytes.len() && bytes[cur] == b')');
                        cur += 1;
                    }
                    b'=' | b'/' => {
                        cur += 1;
                        mv.promoted_to = PgnPiece::from(bytes[cur] as char);
                        assert!(mv.promoted_to != PgnPiece::Unknown);
                        cur += 1;
                    }
                    _ => {}
                }
            }
        }

        assert!(mv.dest.file.is_some());
        assert!(mv.dest.rank.is_some());

        // Check
        mv.check = PgnCheck::__pgn_check_from_string(&s[cur..], &mut cur);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cur..], &mut cur);

        parse_ep_and_nag(s, &mut cur, &mut mv);

        cursor::pgn_cursor_revisit_whitespace(s, &mut cur);
        mv.notation = s[..cur].to_string();
        *consumed += cur;
        mv
    }
}

fn parse_ep_and_nag(s: &str, cur: &mut usize, mv: &mut PgnMove) {
    let bytes = s.as_bytes();

    let skipped_ws_before_ep = cursor::pgn_cursor_skip_whitespace(s, cur);

    // en passant
    if *cur + 1 < bytes.len() && bytes[*cur] == b'e' && bytes[*cur + 1] == b'.' {
        assert!(skipped_ws_before_ep);
        assert!(bytes[*cur] == b'e');
        *cur += 1;
        assert!(bytes[*cur] == b'.');
        *cur += 1;
        assert!(bytes[*cur] == b'p');
        *cur += 1;
        assert!(bytes[*cur] == b'.');
        *cur += 1;
        mv.en_passant = true;
    }

    let skipped_ws_after_ep = if mv.en_passant {
        cursor::pgn_cursor_skip_whitespace(s, cur)
    } else {
        skipped_ws_before_ep
    };

    // NAG annotation
    if mv.annotation == PgnAnnotation::Unknown {
        mv.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[*cur..], cur);
        if mv.annotation != PgnAnnotation::Unknown {
            assert!(skipped_ws_after_ep);
        }
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
                append_check_and_annotation(&mut result, self);
                return write!(f, "{}", result);
            }
            PGN_CASTLING_QUEENSIDE => {
                result.push_str("O-O-O");
                append_check_and_annotation(&mut result, self);
                return write!(f, "{}", result);
            }
            _ => {}
        }

        assert!(self.piece != PgnPiece::Unknown);
        if self.piece != PgnPiece::Pawn {
            result.push(self.piece as u8 as char);
        }

        if let Some(file) = self.from.file {
            result.push(file);
        }
        if let Some(rank) = self.from.rank {
            result.push((b'0' + rank as u8) as char);
        }

        if self.captures {
            result.push('x');
        }

        result.push(self.dest.file.unwrap());
        result.push((b'0' + self.dest.rank.unwrap() as u8) as char);

        if self.promoted_to != PgnPiece::Unknown {
            assert!(self.piece == PgnPiece::Pawn);
            assert!(self.promoted_to != PgnPiece::Pawn);
            result.push('=');
            result.push(self.promoted_to as u8 as char);
        }

        append_check_and_annotation(&mut result, self);
        write!(f, "{}", result)
    }
}

fn append_check_and_annotation(result: &mut String, mv: &PgnMove) {
    match mv.check {
        PgnCheck::None => {}
        PgnCheck::Mate => result.push('#'),
        PgnCheck::Single => result.push('+'),
        PgnCheck::Double => { result.push('+'); result.push('+'); }
    }

    // Inline annotations (GoodMove..DubiousMove)
    let ann_val = mv.annotation as i8;
    if ann_val >= PgnAnnotation::GoodMove as i8 && ann_val <= PgnAnnotation::DubiousMove as i8 {
        result.push_str(&mv.annotation.to_string());
    }

    if mv.en_passant {
        result.push(' ');
        result.push_str("e.p.");
    }

    // NAG annotations (Null or > DubiousMove)
    if ann_val > PgnAnnotation::DubiousMove as i8 || ann_val == PgnAnnotation::Null as i8 {
        result.push(' ');
        result.push_str(&mv.annotation.to_string());
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
        let mut moves = PgnMoves::new();
        pgn_moves_from_string_recurse(s, consumed, &mut moves, PGN_EXPECT_WHITE);
        moves
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
        let mut cur = 0usize;

        while cur < bytes.len() && bytes[cur] == b'(' {
            cur += 1;

            if alt.is_none() {
                *alt = Some(PgnAlternativeMoves::new());
            }

            cursor::pgn_cursor_skip_whitespace(s, &mut cur);
            let mut inner_consumed = 0usize;
            let mut inner_moves = PgnMoves::new();
            pgn_moves_from_string_recurse(&s[cur..], &mut inner_consumed, &mut inner_moves, expect);
            cur += inner_consumed;
            cursor::pgn_cursor_skip_whitespace(s, &mut cur);
            assert!(cur < bytes.len() && bytes[cur] == b')');
            cur += 1;

            alt.as_mut().unwrap().push(inner_moves);

            cursor::pgn_cursor_skip_whitespace(s, &mut cur);

            // Poll comments after alternative
            if placeholder.is_none() {
                *placeholder = Some(PgnComments::new());
            }
            let polled = placeholder.as_mut().unwrap().poll(PgnCommentPosition::AfterAlternative, &s[cur..]);
            cur += polled;
        }

        cur
    }
    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }
}

fn poll_comments(comments: &mut Option<PgnComments>, pos: PgnCommentPosition, s: &str) -> usize {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return 0;
    }
    if comments.is_none() {
        *comments = Some(PgnComments::new());
    }
    comments.as_mut().unwrap().poll(pos, s)
}

fn pgn_moves_from_string_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' {
        return;
    }

    let mut cur = 0usize;
    let mut comments: Option<PgnComments> = None;

    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::BeforeMove, &s[cur..]);

    assert!(cur < bytes.len() && (bytes[cur] as char).is_ascii_digit());
    while cur < bytes.len() && (bytes[cur] as char).is_ascii_digit() {
        cur += 1;
    }

    let mut dots_count = 0;
    assert!(cur < bytes.len() && bytes[cur] == b'.');
    while cur < bytes.len() && bytes[cur] == b'.' {
        cur += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE { assert!(dots_count == 1); }
    if expect == PGN_EXPECT_BLACK { assert!(dots_count == 3); }

    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::BetweenMove, &s[cur..]);

    if dots_count == 3 {
        // Black move only
        let mut black = PgnMove::from_string_with_consumption(&s[cur..], &mut cur);

        cursor::pgn_cursor_skip_whitespace(s, &mut cur);
        cur += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cur..]);
        cur += PgnAlternativeMoves::poll(&mut black.alternatives, &mut comments, &s[cur..], PGN_EXPECT_BLACK);
        cursor::pgn_cursor_skip_whitespace(s, &mut cur);
        cur += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cur..]);

        if let Some(c) = comments.take() {
            black.comments = Some(c);
        }

        moves.push(PgnMovesItem { white: PgnMove::default(), black });
        pgn_moves_from_string_recurse(&s[cur..], &mut cur, moves, PGN_EXPECT_WHITE);
        *consumed += cur;
        return;
    }

    // White move
    let mut white = PgnMove::from_string_with_consumption(&s[cur..], &mut cur);
    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cur..]);
    cur += PgnAlternativeMoves::poll(&mut white.alternatives, &mut comments, &s[cur..], PGN_EXPECT_WHITE);
    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cur..]);

    if let Some(c) = comments.take() {
        white.comments = Some(c);
    }

    // Check if score follows (end of game)
    if PgnScore::from(&s[cur..]) != PgnScore::Unknown {
        moves.push(PgnMovesItem { white, black: PgnMove::default() });
        *consumed += cur;
        return;
    }

    // End of string or end of alternative
    if cur >= bytes.len() || bytes[cur] == b')' {
        moves.push(PgnMovesItem { white, black: PgnMove::default() });
        *consumed += cur;
        return;
    }

    // Parse black move
    let mut comments: Option<PgnComments> = None;

    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::BeforeMove, &s[cur..]);

    // Optional move number for black (e.g., "1...")
    if cur < bytes.len() && (bytes[cur] as char).is_ascii_digit() {
        while cur < bytes.len() && (bytes[cur] as char).is_ascii_digit() {
            cur += 1;
        }
        for _ in 0..3 {
            assert!(cur < bytes.len() && bytes[cur] == b'.');
            cur += 1;
        }
    }

    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::BetweenMove, &s[cur..]);

    let mut black = PgnMove::from_string_with_consumption(&s[cur..], &mut cur);
    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cur..]);
    cur += PgnAlternativeMoves::poll(&mut black.alternatives, &mut comments, &s[cur..], PGN_EXPECT_BLACK);
    cursor::pgn_cursor_skip_whitespace(s, &mut cur);
    cur += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cur..]);

    if let Some(c) = comments.take() {
        black.comments = Some(c);
    }

    moves.push(PgnMovesItem { white, black });

    if PgnScore::from(&s[cur..]) != PgnScore::Unknown {
        *consumed += cur;
        return;
    }

    pgn_moves_from_string_recurse(&s[cur..], &mut cur, moves, PGN_EXPECT_WHITE);
    *consumed += cur;
}
