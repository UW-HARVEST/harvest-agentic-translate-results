use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::{PgnComments, PgnCommentPosition},
    coordinate::PgnCoordinate, piece::PgnPiece, score::PgnScore,
    utils::cursor::{pgn_cursor_skip_whitespace, pgn_cursor_revisit_whitespace},
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
        let mut cursor = 0usize;
        let mut mv = PgnMove::default();

        // Castling
        if cursor < bytes.len() && bytes[cursor] == b'O' {
            cursor += 1;
            assert!(bytes[cursor] == b'-'); cursor += 1;
            assert!(bytes[cursor] == b'O'); cursor += 1;
            mv.castles = PGN_CASTLING_KINGSIDE;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert!(bytes[cursor] == b'O'); cursor += 1;
                mv.castles = PGN_CASTLING_QUEENSIDE;
            }

            // goto check
            mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
            mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

            let skipped_ws_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);
            // en passant check (won't happen for castling but follow C logic)
            if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
                assert!(skipped_ws_before_ep);
                assert!(bytes[cursor] == b'e');
                cursor += 1; assert!(bytes[cursor] == b'.');
                cursor += 1; assert!(bytes[cursor] == b'p');
                cursor += 1; assert!(bytes[cursor] == b'.');
                cursor += 1;
                mv.en_passant = true;
            }

            let skipped_ws_after_ep = if mv.en_passant { pgn_cursor_skip_whitespace(s, &mut cursor) } else { skipped_ws_before_ep };

            if mv.annotation == PgnAnnotation::Unknown {
                mv.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
                if mv.annotation != PgnAnnotation::Unknown {
                    assert!(skipped_ws_after_ep);
                }
            }

            pgn_cursor_revisit_whitespace(s, &mut cursor);
            mv.notation = s[..cursor].to_string();
            *consumed += cursor;
            return mv;
        }

        // Piece
        if cursor < bytes.len() {
            mv.piece = PgnPiece::from(bytes[cursor] as char);
            if mv.piece == PgnPiece::Unknown {
                mv.piece = PgnPiece::Pawn;
            } else {
                cursor += 1;
            }
        }

        // from coordinates
        if cursor < bytes.len() && bytes[cursor].is_ascii_lowercase() && bytes[cursor] != b'x' {
            mv.from.file = Some(bytes[cursor] as char);
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            mv.from.rank = Some((bytes[cursor] - b'0') as i32);
            cursor += 1;
        }

        // captures
        if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
            mv.captures = true;
            cursor += 1;
        }

        // dest coordinates
        if cursor < bytes.len() && bytes[cursor].is_ascii_lowercase() {
            mv.dest.file = Some(bytes[cursor] as char);
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor].is_ascii_digit());
            mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
            cursor += 1;
        } else {
            mv.dest = mv.from;
            mv.from = PgnCoordinate { file: None, rank: None };
        }

        // promotion
        if cursor < bytes.len() {
            mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
            if mv.promoted_to == PgnPiece::Unknown {
                match bytes[cursor] {
                    b'(' => {
                        cursor += 1;
                        mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                        assert!(mv.promoted_to != PgnPiece::Unknown);
                        cursor += 1;
                        assert!(bytes[cursor] == b')');
                        cursor += 1;
                    }
                    b'=' | b'/' => {
                        cursor += 1;
                        mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                        assert!(mv.promoted_to != PgnPiece::Unknown);
                        cursor += 1;
                    }
                    _ => {}
                }
            } else {
                cursor += 1;
            }
        }

        assert!(mv.dest.file.is_some());
        assert!(mv.dest.rank.is_some());

        // check
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_ws_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // en passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            assert!(skipped_ws_before_ep);
            assert!(bytes[cursor] == b'e');
            cursor += 1; assert!(bytes[cursor] == b'.');
            cursor += 1; assert!(bytes[cursor] == b'p');
            cursor += 1; assert!(bytes[cursor] == b'.');
            cursor += 1;
            mv.en_passant = true;
        }

        let skipped_ws_after_ep = if mv.en_passant { pgn_cursor_skip_whitespace(s, &mut cursor) } else { skipped_ws_before_ep };

        // NAG annotation
        if mv.annotation == PgnAnnotation::Unknown {
            mv.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if mv.annotation != PgnAnnotation::Unknown {
                assert!(skipped_ws_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);
        mv.notation = s[..cursor].to_string();
        *consumed += cursor;
        mv
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
        let mut buf = String::new();

        match self.castles {
            PGN_CASTLING_KINGSIDE => {
                buf.push_str("O-O");
                append_check_annotation_ep(&mut buf, self);
                // Set notation as side effect (mirrors C's pgn_move_to_string writing to move.notation)
                unsafe {
                    let self_mut = self as *const PgnMove as *mut PgnMove;
                    (*self_mut).notation = buf.clone();
                }
                return write!(f, "{}", buf);
            }
            PGN_CASTLING_QUEENSIDE => {
                buf.push_str("O-O-O");
                append_check_annotation_ep(&mut buf, self);
                unsafe {
                    let self_mut = self as *const PgnMove as *mut PgnMove;
                    (*self_mut).notation = buf.clone();
                }
                return write!(f, "{}", buf);
            }
            _ => {}
        }

        assert!(self.piece != PgnPiece::Unknown);
        if self.piece != PgnPiece::Pawn {
            buf.push(self.piece as u8 as char);
        }

        if let Some(file) = self.from.file { buf.push(file); }
        if let Some(rank) = self.from.rank { buf.push_str(&format!("{}", rank)); }

        if self.captures { buf.push('x'); }

        if let Some(file) = self.dest.file { buf.push(file); }
        if let Some(rank) = self.dest.rank { buf.push_str(&format!("{}", rank)); }

        if self.promoted_to != PgnPiece::Unknown {
            assert!(self.piece == PgnPiece::Pawn);
            assert!(self.promoted_to != PgnPiece::Pawn);
            buf.push('=');
            buf.push(self.promoted_to as u8 as char);
        }

        append_check_annotation_ep(&mut buf, self);

        unsafe {
            let self_mut = self as *const PgnMove as *mut PgnMove;
            (*self_mut).notation = buf.clone();
        }

        write!(f, "{}", buf)
    }
}

fn append_check_annotation_ep(buf: &mut String, mv: &PgnMove) {
    match mv.check {
        PgnCheck::None => {}
        PgnCheck::Mate => buf.push('#'),
        PgnCheck::Single => buf.push('+'),
        PgnCheck::Double => { buf.push('+'); buf.push('+'); }
    }

    let nag = mv.annotation.nag_value();
    if nag >= 1 && nag <= 6 {
        buf.push_str(&mv.annotation.to_string());
    }

    if mv.en_passant {
        buf.push_str(" e.p.");
    }

    if nag > 6 || nag == 0 {
        buf.push(' ');
        buf.push_str(&format!("${}", nag));
    }
}

impl PgnAnnotation {
    pub fn nag_value(&self) -> i8 {
        // With #[repr(i8)], the discriminant is the i8 value
        // This works for both known variants and transmuted raw values
        unsafe { *(self as *const PgnAnnotation as *const i8) }
    }
}

impl PartialOrd for PgnAnnotation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.nag_value().cmp(&other.nag_value()))
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
        let mut cursor = 0usize;

        while cursor < bytes.len() && bytes[cursor] == b'(' {
            cursor += 1;

            if alt.is_none() {
                *alt = Some(PgnAlternativeMoves::new());
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);
            let mut inner_moves = PgnMoves::new();
            pgn_moves_from_string_recurse(&s[cursor..], &mut cursor, &mut inner_moves, expect);
            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            alt.as_mut().unwrap().push(inner_moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            // poll comments with AfterAlternative position
            if let Some(ref mut ph) = placeholder {
                cursor += ph.poll(PgnCommentPosition::AfterAlternative, &s[cursor..]);
            } else {
                let mut temp = PgnComments::new();
                let adv = temp.poll(PgnCommentPosition::AfterAlternative, &s[cursor..]);
                if !temp.values.is_empty() {
                    cursor += adv;
                    *placeholder = Some(temp);
                }
            }
        }

        cursor
    }
    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }
}

fn pgn_moves_from_string_recurse(str: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = str.as_bytes();
    let full_str = str;

    if bytes.is_empty() || bytes[0] == b')' {
        return;
    }

    let mut cursor = 0usize;

    // comments placeholder
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::BeforeMove, &full_str[cursor..]);
        cursor += adv;
    }

    assert!(cursor < bytes.len() && bytes[cursor].is_ascii_digit());
    while cursor < bytes.len() && bytes[cursor].is_ascii_digit() { cursor += 1; }

    let mut dots_count = 0;
    assert!(cursor < bytes.len() && bytes[cursor] == b'.');
    while cursor < bytes.len() && bytes[cursor] == b'.' {
        cursor += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE { assert!(dots_count == 1); }
    if expect == PGN_EXPECT_BLACK { assert!(dots_count == 3); }

    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::BetweenMove, &full_str[cursor..]);
        cursor += adv;
    }

    if dots_count == 3 {
        let black = PgnMove::from_string_with_consumption(&full_str[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(full_str, &mut cursor);
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &full_str[cursor..]);
        cursor += adv;

        let mut black_alts: Option<PgnAlternativeMoves> = None;
        let alt_adv = PgnAlternativeMoves::poll(&mut black_alts, &mut comments, &full_str[cursor..], PGN_EXPECT_BLACK);
        cursor += alt_adv;
        pgn_cursor_skip_whitespace(full_str, &mut cursor);
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &full_str[cursor..]);
        cursor += adv;

        let mut item = PgnMovesItem {
            white: PgnMove::default(),
            black,
        };
        item.black.alternatives = black_alts;
        if let Some(c) = comments.take() {
            item.black.comments = Some(c);
        }

        moves.push(item);
        pgn_moves_from_string_recurse(&full_str[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    let white = PgnMove::from_string_with_consumption(&full_str[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &full_str[cursor..]);
        cursor += adv;
    }

    let mut white_alts: Option<PgnAlternativeMoves> = None;
    let alt_adv = PgnAlternativeMoves::poll(&mut white_alts, &mut comments, &full_str[cursor..], PGN_EXPECT_WHITE);
    cursor += alt_adv;
    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &full_str[cursor..]);
        cursor += adv;
    }

    let mut white_move = white;
    white_move.alternatives = white_alts;
    if let Some(c) = comments.take() {
        white_move.comments = Some(c);
    }

    // Check for score
    if cursor < bytes.len() && PgnScore::from(&full_str[cursor..]) != PgnScore::Unknown {
        moves.push(PgnMovesItem { white: white_move, black: PgnMove::default() });
        *consumed += cursor;
        return;
    }

    // End of string or closing paren
    if cursor >= bytes.len() || bytes[cursor] == b')' || bytes[cursor] == b'\0' {
        moves.push(PgnMovesItem { white: white_move, black: PgnMove::default() });
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::BeforeMove, &full_str[cursor..]);
        cursor += adv;
    }

    // Optional black move number
    if cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() { cursor += 1; }
        for _ in 0..3 {
            assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
        }
    }

    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::BetweenMove, &full_str[cursor..]);
        cursor += adv;
    }

    let black = PgnMove::from_string_with_consumption(&full_str[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &full_str[cursor..]);
        cursor += adv;
    }

    let mut black_alts: Option<PgnAlternativeMoves> = None;
    let alt_adv = PgnAlternativeMoves::poll(&mut black_alts, &mut comments, &full_str[cursor..], PGN_EXPECT_BLACK);
    cursor += alt_adv;
    pgn_cursor_skip_whitespace(full_str, &mut cursor);
    {
        let adv = poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &full_str[cursor..]);
        cursor += adv;
    }

    let mut black_move = black;
    black_move.alternatives = black_alts;
    if let Some(c) = comments.take() {
        black_move.comments = Some(c);
    }

    moves.push(PgnMovesItem { white: white_move, black: black_move });

    if cursor < bytes.len() && PgnScore::from(&full_str[cursor..]) != PgnScore::Unknown {
        *consumed += cursor;
        return;
    }

    pgn_moves_from_string_recurse(&full_str[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}

/// Helper to poll comments into an Option<PgnComments>, creating it if needed
fn poll_comments_into(comments: &mut Option<PgnComments>, pos: PgnCommentPosition, s: &str) -> usize {
    if s.as_bytes().first() != Some(&b'{') {
        return 0;
    }
    if comments.is_none() {
        *comments = Some(PgnComments::new());
    }
    comments.as_mut().unwrap().poll(pos, s)
}
