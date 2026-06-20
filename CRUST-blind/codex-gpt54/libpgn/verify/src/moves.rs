use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::PgnComments, coordinate::PgnCoordinate,
    piece::PgnPiece,
};
const PGN_EXPECT_WHITE: i32 = 0;
const PGN_EXPECT_BLACK: i32 = 1;

fn skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let start = *cursor;
    while matches!(bytes.get(*cursor), Some(b) if (*b as char).is_ascii_whitespace()) {
        *cursor += 1;
    }
    *cursor != start
}

fn revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let start = *cursor;
    while *cursor > 0
        && matches!(bytes.get(*cursor - 1), Some(b) if (*b as char).is_ascii_whitespace())
    {
        *cursor -= 1;
    }
    *cursor != start
}

fn poll_comments(target: &mut Option<PgnComments>, pos: crate::comments::PgnCommentPosition, s: &str) -> usize {
    if !s.starts_with('{') {
        return 0;
    }

    if target.is_none() {
        *target = Some(PgnComments::new());
    }

    target.as_mut().map_or(0, |comments| comments.poll(pos, s))
}

fn parse_moves_recurse(s: &str, consumed: &mut usize, mut moves: PgnMoves, expect: i32) -> PgnMoves {
    let bytes = s.as_bytes();
    if matches!(bytes.first(), Some(b')') | Some(b'\0')) || s.is_empty() {
        return moves;
    }

    let mut cursor = 0;
    let mut item = PgnMovesItem {
        white: PgnMove::default(),
        black: PgnMove::default(),
    };
    let mut comments: Option<PgnComments> = None;

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::BeforeMove, &s[cursor..]);

    while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
        cursor += 1;
    }

    let mut dots_count = 0;
    while matches!(bytes.get(cursor), Some(b'.')) {
        cursor += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE && dots_count != 1 {
        *consumed += cursor;
        return moves;
    }
    if expect == PGN_EXPECT_BLACK && dots_count != 3 {
        *consumed += cursor;
        return moves;
    }

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
        skip_whitespace(s, &mut cursor);
        cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::AfterMove, &s[cursor..]);
        cursor += PgnAlternativeMoves::poll(&mut item.black.alternatives, &mut comments, &s[cursor..], PGN_EXPECT_BLACK);
        skip_whitespace(s, &mut cursor);
        cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::AfterMove, &s[cursor..]);

        if let Some(comments) = comments.take() {
            item.black.comments = Some(comments);
        }

        moves.push(item);
        let mut inner = 0;
        moves = parse_moves_recurse(&s[cursor..], &mut inner, moves, PGN_EXPECT_WHITE);
        cursor += inner;
        *consumed += cursor;
        return moves;
    }

    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(&mut item.white.alternatives, &mut comments, &s[cursor..], PGN_EXPECT_WHITE);
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::AfterMove, &s[cursor..]);

    if let Some(comments) = comments.take() {
        item.white.comments = Some(comments);
    }

    if crate::score::PgnScore::from(&s[cursor..]) != crate::score::PgnScore::Unknown {
        moves.push(item);
        *consumed += cursor;
        return moves;
    }

    if s[cursor..].is_empty() || matches!(bytes.get(cursor), Some(b')')) {
        moves.push(item);
        *consumed += cursor;
        return moves;
    }

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::BeforeMove, &s[cursor..]);

    if matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
        while matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
            cursor += 1;
        }
        if s[cursor..].starts_with("...") {
            cursor += 3;
        }
    }

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::BetweenMove, &s[cursor..]);

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(&mut item.black.alternatives, &mut comments, &s[cursor..], PGN_EXPECT_BLACK);
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, crate::comments::PgnCommentPosition::AfterMove, &s[cursor..]);

    if let Some(comments) = comments.take() {
        item.black.comments = Some(comments);
    }

    moves.push(item);

    if crate::score::PgnScore::from(&s[cursor..]) != crate::score::PgnScore::Unknown {
        *consumed += cursor;
        return moves;
    }

    let mut inner = 0;
    moves = parse_moves_recurse(&s[cursor..], &mut inner, moves, PGN_EXPECT_WHITE);
    cursor += inner;
    *consumed += cursor;
    moves
}

pub const PGN_CASTLING_NONE: u8 = 0;
pub const PGN_CASTLING_KINGSIDE: u8 = 2;
pub const PGN_CASTLING_QUEENSIDE: u8 = 3;
pub const PGN_MOVES_INITIAL_SIZE: usize = 32;
pub const PGN_MOVES_GROW_SIZE: usize = 32;
pub const PGN_MOVE_NOTATION_SIZE: usize = 16;
pub const PGN_ALTERNATIVE_MOVES_INITIAL_SIZE: usize = 1;
pub const PGN_ALTERNATIVE_MOVES_GROW_SIZE: usize = 1;
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
        Self {
            piece: PgnPiece::Unknown,
            promoted_to: PgnPiece::Unknown,
            notation: String::new(),
            castles: PGN_CASTLING_NONE,
            captures: false,
            en_passant: false,
            check: PgnCheck::None,
            from: PgnCoordinate {
                file: None,
                rank: None,
            },
            dest: PgnCoordinate {
                file: None,
                rank: None,
            },
            annotation: PgnAnnotation::Unknown,
            comments: None,
            alternatives: None,
        }
    }
}
impl PgnMove {
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut mv = Self::default();
        let mut cursor = 0;

        if matches!(bytes.get(cursor), Some(b'O')) {
            cursor += 1;
            if matches!(bytes.get(cursor), Some(b'-')) && matches!(bytes.get(cursor + 1), Some(b'O')) {
                cursor += 2;
                mv.castles = PGN_CASTLING_KINGSIDE;
                if matches!(bytes.get(cursor), Some(b'-')) && matches!(bytes.get(cursor + 1), Some(b'O')) {
                    cursor += 2;
                    mv.castles = PGN_CASTLING_QUEENSIDE;
                }
            }
        } else {
            if let Some(ch) = bytes.get(cursor).map(|byte| *byte as char) {
                mv.piece = PgnPiece::from(ch);
                if mv.piece != PgnPiece::Unknown {
                    cursor += 1;
                } else {
                    mv.piece = PgnPiece::Pawn;
                }
            }

            if matches!(bytes.get(cursor), Some(b'a'..=b'z')) && !matches!(bytes.get(cursor), Some(b'x')) {
                mv.from.file = bytes.get(cursor).map(|byte| *byte as char);
                cursor += 1;
            }
            if matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
                mv.from.rank = bytes.get(cursor).map(|byte| (byte - b'0') as i32);
                cursor += 1;
            }

            mv.captures = matches!(bytes.get(cursor), Some(b'x' | b':'));
            if mv.captures {
                cursor += 1;
            }

            if matches!(bytes.get(cursor), Some(b'a'..=b'z')) {
                mv.dest.file = bytes.get(cursor).map(|byte| *byte as char);
                cursor += 1;
                if matches!(bytes.get(cursor), Some(b'0'..=b'9')) {
                    mv.dest.rank = bytes.get(cursor).map(|byte| (byte - b'0') as i32);
                    cursor += 1;
                }
            } else {
                mv.dest = mv.from;
                mv.from = PgnCoordinate {
                    file: None,
                    rank: None,
                };
            }

            mv.promoted_to = bytes
                .get(cursor)
                .map(|byte| PgnPiece::from(*byte as char))
                .unwrap_or(PgnPiece::Unknown);

            if mv.promoted_to == PgnPiece::Unknown {
                match bytes.get(cursor).copied() {
                    Some(b'(') => {
                        cursor += 1;
                        mv.promoted_to = bytes
                            .get(cursor)
                            .map(|byte| PgnPiece::from(*byte as char))
                            .unwrap_or(PgnPiece::Unknown);
                        if mv.promoted_to != PgnPiece::Unknown {
                            cursor += 1;
                        }
                        if matches!(bytes.get(cursor), Some(b')')) {
                            cursor += 1;
                        }
                    }
                    Some(b'=' | b'/') => {
                        cursor += 1;
                        mv.promoted_to = bytes
                            .get(cursor)
                            .map(|byte| PgnPiece::from(*byte as char))
                            .unwrap_or(PgnPiece::Unknown);
                        if mv.promoted_to != PgnPiece::Unknown {
                            cursor += 1;
                        }
                    }
                    _ => {}
                }
            } else {
                cursor += 1;
            }
        }

        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = skip_whitespace(s, &mut cursor);
        if matches!(bytes.get(cursor), Some(b'e')) && matches!(bytes.get(cursor + 1), Some(b'.')) {
            cursor += 4.min(s.len().saturating_sub(cursor));
            mv.en_passant = true;
        }

        let skipped_whitespace_after_ep = if mv.en_passant {
            skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        if mv.annotation == PgnAnnotation::Unknown {
            let annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if annotation != PgnAnnotation::Unknown && skipped_whitespace_after_ep {
                mv.annotation = annotation;
            }
        }

        revisit_whitespace(s, &mut cursor);
        mv.notation = s[..cursor].to_string();
        *consumed += cursor;
        mv
    }
}
impl From<&str> for PgnMove {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        Self::from_string_with_consumption(s, &mut consumed)
    }
}
impl Display for PgnMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        match self.castles {
            PGN_CASTLING_NONE => {
                if self.piece != PgnPiece::Unknown && self.piece != PgnPiece::Pawn {
                    out.push(self.piece as u8 as char);
                }
                if let Some(file) = self.from.file {
                    out.push(file);
                }
                if let Some(rank) = self.from.rank {
                    out.push(char::from_digit(rank as u32, 10).unwrap_or('0'));
                }
                if self.captures {
                    out.push('x');
                }
                if let Some(file) = self.dest.file {
                    out.push(file);
                }
                if let Some(rank) = self.dest.rank {
                    out.push(char::from_digit(rank as u32, 10).unwrap_or('0'));
                }
                if self.promoted_to != PgnPiece::Unknown {
                    out.push('=');
                    out.push(self.promoted_to as u8 as char);
                }
            }
            PGN_CASTLING_KINGSIDE => out.push_str("O-O"),
            PGN_CASTLING_QUEENSIDE => out.push_str("O-O-O"),
            _ => {}
        }

        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => out.push('#'),
            PgnCheck::Single => out.push('+'),
            PgnCheck::Double => out.push_str("++"),
        }

        match self.annotation {
            PgnAnnotation::GoodMove
            | PgnAnnotation::Mistake
            | PgnAnnotation::BrilliantMove
            | PgnAnnotation::Blunder
            | PgnAnnotation::InterestingMove
            | PgnAnnotation::DubiousMove => out.push_str(&self.annotation.to_string()),
            _ => {}
        }

        if self.en_passant {
            out.push_str(" e.p.");
        }

        if self.annotation == PgnAnnotation::Null {
            out.push(' ');
            out.push_str(&self.annotation.to_string());
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
        let mut consumed = 0;
        Self::from_string_with_consumption(s, &mut consumed)
    }
}
impl PgnMoves {
    pub fn new() -> Self {
        Self {
            values: Vec::with_capacity(PGN_MOVES_INITIAL_SIZE),
        }
    }
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        parse_moves_recurse(s, consumed, Self::new(), PGN_EXPECT_WHITE)
    }
    pub fn push(&mut self, moves: PgnMovesItem) {
        if self.values.len() == self.values.capacity() {
            self.values.reserve(PGN_MOVES_GROW_SIZE);
        }
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
        Self {
            values: Vec::with_capacity(PGN_ALTERNATIVE_MOVES_INITIAL_SIZE),
        }
    }
    pub fn poll(
        alt: &mut Option<Self>,
        placeholder: &mut Option<PgnComments>,
        s: &str,
        expect: i32,
    ) -> usize {
        let bytes = s.as_bytes();
        let mut cursor = 0;

        while matches!(bytes.get(cursor), Some(b'(')) {
            cursor += 1;
            if alt.is_none() {
                *alt = Some(Self::new());
            }

            skip_whitespace(s, &mut cursor);
            let mut inner = 0;
            let moves = parse_moves_recurse(&s[cursor..], &mut inner, PgnMoves::new(), expect);
            cursor += inner;

            if let Some(alternatives) = alt.as_mut() {
                alternatives.push(moves);
            }

            skip_whitespace(s, &mut cursor);
            if matches!(bytes.get(cursor), Some(b')')) {
                cursor += 1;
            }

            skip_whitespace(s, &mut cursor);
            cursor += poll_comments(
                placeholder,
                crate::comments::PgnCommentPosition::AfterAlternative,
                &s[cursor..],
            );
        }

        cursor
    }
    pub fn push(&mut self, moves: PgnMoves) {
        if self.values.len() == self.values.capacity() {
            self.values.reserve(PGN_ALTERNATIVE_MOVES_GROW_SIZE);
        }
        self.values.push(Box::new(moves));
    }
}
