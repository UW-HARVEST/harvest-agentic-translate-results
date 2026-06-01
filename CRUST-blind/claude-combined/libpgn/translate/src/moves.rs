use std::fmt::Display;

use crate::utils::cursor::{
    pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace,
};
use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{PgnCommentPosition, PgnComments},
    coordinate::PgnCoordinate,
    piece::PgnPiece,
    score::PgnScore,
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
        let mut move_data = PgnMove::default();
        let mut cursor: usize = 0;

        let mut at_check_label = false;

        if cursor < bytes.len() && bytes[cursor] == b'O' {
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'-');
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'O');
            cursor += 1;
            move_data.castles = PGN_CASTLING_KINGSIDE;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert!(cursor < bytes.len() && bytes[cursor] == b'O');
                cursor += 1;
                move_data.castles = PGN_CASTLING_QUEENSIDE;
            }

            at_check_label = true;
        }

        if !at_check_label {
            if cursor < bytes.len() {
                move_data.piece = PgnPiece::from(bytes[cursor] as char);
                cursor += 1;
                if move_data.piece == PgnPiece::Unknown {
                    move_data.piece = PgnPiece::Pawn;
                    cursor -= 1;
                }
            }

            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                if c.is_ascii_lowercase() && c != 'x' {
                    move_data.from.file = Some(c);
                    cursor += 1;
                }
            }

            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                if c.is_ascii_digit() {
                    move_data.from.rank = Some((c as u8 - b'0') as i32);
                    cursor += 1;
                }
            }

            if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
                move_data.captures = true;
                cursor += 1;
            }

            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                move_data.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
                move_data.dest.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            } else {
                move_data.dest = move_data.from;
                move_data.from = PgnCoordinate { file: None, rank: None };
            }

            if cursor < bytes.len() {
                let promo = PgnPiece::from(bytes[cursor] as char);
                move_data.promoted_to = promo;
                if promo == PgnPiece::Unknown {
                    if bytes[cursor] == b'(' {
                        cursor += 1;
                        assert!(cursor < bytes.len());
                        let p = PgnPiece::from(bytes[cursor] as char);
                        move_data.promoted_to = p;
                        assert!(p != PgnPiece::Unknown);
                        cursor += 1;
                        assert!(cursor < bytes.len() && bytes[cursor] == b')');
                        cursor += 1;
                    } else if bytes[cursor] == b'=' || bytes[cursor] == b'/' {
                        cursor += 1;
                        assert!(cursor < bytes.len());
                        let p = PgnPiece::from(bytes[cursor] as char);
                        move_data.promoted_to = p;
                        assert!(p != PgnPiece::Unknown);
                        cursor += 1;
                    }
                } else {
                    cursor += 1;
                }
            }

            assert!(move_data.dest.file.is_some());
            assert!(move_data.dest.rank.is_some());
        }

        // Parse trailing check, annotation, en-passant, NAG.
        let mut local_consumed = 0usize;
        move_data.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut local_consumed);
        cursor += local_consumed;

        local_consumed = 0;
        move_data.annotation =
            PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut local_consumed);
        cursor += local_consumed;

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            assert!(skipped_whitespace_before_ep);

            assert!(bytes[cursor] == b'e');
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'p');
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
            move_data.en_passant = true;
        }

        let skipped_whitespace_after_ep = if move_data.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        if move_data.annotation == PgnAnnotation::Unknown {
            local_consumed = 0;
            move_data.annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut local_consumed);
            cursor += local_consumed;

            if move_data.annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        let notation_len = cursor;
        move_data.notation = s[..notation_len].to_string();

        *consumed += cursor;
        move_data
    }
}

impl From<&str> for PgnMove {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnMove::from_string_with_consumption(s, &mut consumed)
    }
}

impl Display for PgnMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        let mut at_check = false;
        match self.castles {
            PGN_CASTLING_NONE => {}
            PGN_CASTLING_KINGSIDE => {
                out.push_str("O-O");
                at_check = true;
            }
            PGN_CASTLING_QUEENSIDE => {
                out.push_str("O-O-O");
                at_check = true;
            }
            _ => {}
        }

        if !at_check {
            assert!(self.piece != PgnPiece::Unknown);
            if self.piece != PgnPiece::Pawn {
                out.push(self.piece as u8 as char);
            }
            if let Some(file) = self.from.file {
                out.push(file);
            }
            if let Some(rank) = self.from.rank {
                if rank != 0 {
                    out.push((b'0' + rank as u8) as char);
                }
            }
            if self.captures {
                out.push('x');
            }
            if let Some(file) = self.dest.file {
                out.push(file);
            }
            if let Some(rank) = self.dest.rank {
                out.push((b'0' + rank as u8) as char);
            }
            if self.promoted_to != PgnPiece::Unknown {
                assert!(self.piece == PgnPiece::Pawn);
                assert!(self.promoted_to != PgnPiece::Pawn);
                out.push('=');
                out.push(self.promoted_to as u8 as char);
            }
        }

        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => out.push('#'),
            PgnCheck::Single => out.push('+'),
            PgnCheck::Double => {
                out.push('+');
                out.push('+');
            }
        }

        let in_inline_range = matches!(
            self.annotation,
            PgnAnnotation::GoodMove
                | PgnAnnotation::Mistake
                | PgnAnnotation::BrilliantMove
                | PgnAnnotation::Blunder
                | PgnAnnotation::InterestingMove
                | PgnAnnotation::DubiousMove
        );
        if in_inline_range {
            out.push_str(&format!("{}", self.annotation));
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        if matches!(self.annotation, PgnAnnotation::Null) {
            out.push(' ');
            out.push_str(&format!("{}", self.annotation));
        }

        f.write_str(&out)
    }
}

#[derive(Debug)]
pub struct PgnMoves {
    pub values: Vec<PgnMovesItem>,
}

impl From<&str> for PgnMoves {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnMoves::from_string_with_consumption(s, &mut consumed)
    }
}

impl PgnMoves {
    pub fn new() -> Self {
        let _ = (PGN_MOVES_INITIAL_SIZE, PGN_MOVES_GROW_SIZE);
        PgnMoves {
            values: Vec::with_capacity(PGN_MOVES_INITIAL_SIZE),
        }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let mut moves = PgnMoves::new();
        let mut cursor = 0usize;
        moves_recurse(s, &mut cursor, &mut moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        moves
    }

    pub fn push(&mut self, item: PgnMovesItem) {
        self.values.push(item);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl Default for PgnMoves {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PgnMovesItem {
    pub white: PgnMove,
    pub black: PgnMove,
}

impl Default for PgnMovesItem {
    fn default() -> Self {
        PgnMovesItem {
            white: PgnMove::default(),
            black: PgnMove::default(),
        }
    }
}

#[derive(Debug)]
pub struct PgnAlternativeMoves {
    pub values: Vec<Box<PgnMoves>>,
}

impl PgnAlternativeMoves {
    pub fn new() -> Self {
        let _ = (PGN_ALTERNATIVE_MOVES_INITIAL_SIZE, PGN_ALTERNATIVE_MOVES_GROW_SIZE);
        PgnAlternativeMoves {
            values: Vec::with_capacity(PGN_ALTERNATIVE_MOVES_INITIAL_SIZE),
        }
    }

    pub fn poll(
        alt: &mut Option<Self>,
        placeholder: &mut Option<PgnComments>,
        s: &str,
        expect: i32,
    ) -> usize {
        poll_alternatives(alt, placeholder, s, expect)
    }

    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

/// Recursive helper that mirrors the C `__pgn_moves_from_string_recurse`.
/// `s` is the (sub)string to parse from offset 0; `cursor` is advanced
/// by the number of bytes consumed.
fn moves_recurse(s: &str, cursor: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if *cursor >= bytes.len() {
        return;
    }
    if bytes[*cursor] == b')' || bytes[*cursor] == 0 {
        return;
    }

    let mut item = PgnMovesItem::default();
    let mut comments_placeholder: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::BeforeMove,
    );

    assert!(*cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_digit());
    while *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_digit() {
        *cursor += 1;
    }

    let mut dots_count = 0;
    assert!(*cursor < bytes.len() && bytes[*cursor] == b'.');
    while *cursor < bytes.len() && bytes[*cursor] == b'.' {
        *cursor += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE {
        assert_eq!(dots_count, 1);
    }
    if expect == PGN_EXPECT_BLACK {
        assert_eq!(dots_count, 3);
    }

    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::BetweenMove,
    );

    if dots_count == 3 {
        // black move only
        let mut local = 0usize;
        item.black = PgnMove::from_string_with_consumption(&s[*cursor..], &mut local);
        *cursor += local;

        pgn_cursor_skip_whitespace(s, cursor);
        poll_comments(
            &mut comments_placeholder,
            s,
            cursor,
            PgnCommentPosition::AfterMove,
        );
        let added = poll_alternatives(
            &mut item.black.alternatives,
            &mut comments_placeholder,
            &s[*cursor..],
            PGN_EXPECT_BLACK,
        );
        *cursor += added;

        pgn_cursor_skip_whitespace(s, cursor);
        poll_comments(
            &mut comments_placeholder,
            s,
            cursor,
            PgnCommentPosition::AfterMove,
        );

        if let Some(c) = comments_placeholder.take() {
            item.black.comments = Some(c);
        }
        moves.push(item);
        moves_recurse(s, cursor, moves, PGN_EXPECT_WHITE);
        return;
    }

    // Parse white move
    let mut local = 0usize;
    item.white = PgnMove::from_string_with_consumption(&s[*cursor..], &mut local);
    *cursor += local;

    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::AfterMove,
    );
    let added = poll_alternatives(
        &mut item.white.alternatives,
        &mut comments_placeholder,
        &s[*cursor..],
        PGN_EXPECT_WHITE,
    );
    *cursor += added;
    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::AfterMove,
    );

    if let Some(c) = comments_placeholder.take() {
        item.white.comments = Some(c);
    }

    if PgnScore::from(&s[*cursor..]) != PgnScore::Unknown {
        moves.push(item);
        return;
    }

    if *cursor >= bytes.len() || bytes[*cursor] == b')' || bytes[*cursor] == 0 {
        moves.push(item);
        return;
    }

    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::BeforeMove,
    );

    if *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_digit() {
        while *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_digit() {
            *cursor += 1;
        }
        for _ in 0..3 {
            assert!(*cursor < bytes.len() && bytes[*cursor] == b'.');
            *cursor += 1;
        }
    }

    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::BetweenMove,
    );

    let mut local = 0usize;
    item.black = PgnMove::from_string_with_consumption(&s[*cursor..], &mut local);
    *cursor += local;
    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::AfterMove,
    );
    let added = poll_alternatives(
        &mut item.black.alternatives,
        &mut comments_placeholder,
        &s[*cursor..],
        PGN_EXPECT_BLACK,
    );
    *cursor += added;
    pgn_cursor_skip_whitespace(s, cursor);
    poll_comments(
        &mut comments_placeholder,
        s,
        cursor,
        PgnCommentPosition::AfterMove,
    );

    if let Some(c) = comments_placeholder.take() {
        item.black.comments = Some(c);
    }
    moves.push(item);

    if PgnScore::from(&s[*cursor..]) != PgnScore::Unknown {
        return;
    }

    moves_recurse(s, cursor, moves, PGN_EXPECT_WHITE);
}

fn poll_comments(
    placeholder: &mut Option<PgnComments>,
    s: &str,
    cursor: &mut usize,
    pos: PgnCommentPosition,
) {
    let bytes = s.as_bytes();
    if *cursor < bytes.len() && bytes[*cursor] == b'{' {
        if placeholder.is_none() {
            *placeholder = Some(PgnComments::new());
        }
        let added = placeholder
            .as_mut()
            .unwrap()
            .poll(pos, &s[*cursor..]);
        *cursor += added;
    }
}

/// Helper that mirrors `pgn_alternative_moves_poll`.
fn poll_alternatives(
    alt: &mut Option<PgnAlternativeMoves>,
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

        let mut inner = PgnMoves::new();
        let mut inner_cursor = 0usize;
        moves_recurse(&s[cursor..], &mut inner_cursor, &mut inner, expect);
        cursor += inner_cursor;
        alt.as_mut().unwrap().push(inner);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        assert!(cursor < bytes.len() && bytes[cursor] == b')');
        cursor += 1;

        pgn_cursor_skip_whitespace(s, &mut cursor);

        if placeholder.is_none() {
            *placeholder = Some(PgnComments::new());
        }
        let added = placeholder
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::AfterAlternative, &s[cursor..]);
        cursor += added;
    }
    cursor
}
