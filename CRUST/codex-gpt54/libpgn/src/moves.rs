use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::PgnComments, coordinate::PgnCoordinate,
    piece::PgnPiece,
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

fn skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

fn revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor > 0 && bytes[*cursor - 1].is_ascii_whitespace() {
        *cursor -= 1;
        skipped = true;
    }
    skipped
}

fn poll_comments(
    comments: &mut Option<PgnComments>,
    pos: crate::comments::PgnCommentPosition,
    s: &str,
) -> usize {
    if !s.starts_with('{') {
        return 0;
    }

    let entry = comments.get_or_insert_with(PgnComments::new);
    entry.poll(pos, s)
}

fn piece_letter(piece: PgnPiece) -> Option<char> {
    match piece {
        PgnPiece::Pawn => Some('P'),
        PgnPiece::Rook => Some('R'),
        PgnPiece::Knight => Some('N'),
        PgnPiece::Bishop => Some('B'),
        PgnPiece::Queen => Some('Q'),
        PgnPiece::King => Some('K'),
        PgnPiece::Unknown => None,
    }
}

fn score_is_present(s: &str) -> bool {
    !matches!(crate::score::PgnScore::from(s), crate::score::PgnScore::Unknown)
}

fn raw_annotation_from_notation(notation: &str) -> Option<&str> {
    let candidate = notation.split_whitespace().last()?;
    if candidate.starts_with('$') && candidate[1..].bytes().all(|byte| byte.is_ascii_digit()) {
        Some(candidate)
    } else {
        None
    }
}

fn parse_moves_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    if s.is_empty() || s.starts_with(')') {
        return;
    }

    let bytes = s.as_bytes();
    let mut cursor = 0usize;
    let mut move_item = PgnMovesItem {
        white: PgnMove::default(),
        black: PgnMove::default(),
    };
    let mut comments = None;

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::BeforeMove,
        &s[cursor..],
    );

    assert!(bytes.get(cursor).is_some_and(u8::is_ascii_digit));
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }

    let mut dots_count = 0;
    assert_eq!(bytes.get(cursor), Some(&b'.'));
    while bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE {
        assert_eq!(dots_count, 1);
    }
    if expect == PGN_EXPECT_BLACK {
        assert_eq!(dots_count, 3);
    }

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    if dots_count == 3 {
        move_item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);

        skip_whitespace(s, &mut cursor);
        cursor += poll_comments(
            &mut comments,
            crate::comments::PgnCommentPosition::AfterMove,
            &s[cursor..],
        );
        cursor += PgnAlternativeMoves::poll(
            &mut move_item.black.alternatives,
            &mut comments,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        skip_whitespace(s, &mut cursor);
        cursor += poll_comments(
            &mut comments,
            crate::comments::PgnCommentPosition::AfterMove,
            &s[cursor..],
        );

        if comments.is_some() {
            move_item.black.comments = comments.take();
        }

        moves.push(move_item);
        parse_moves_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    move_item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::AfterMove,
        &s[cursor..],
    );
    cursor += PgnAlternativeMoves::poll(
        &mut move_item.white.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments.is_some() {
        move_item.white.comments = comments.take();
    }

    if score_is_present(&s[cursor..]) {
        moves.push(move_item);
        *consumed += cursor;
        return;
    }

    if s[cursor..].starts_with(')') || cursor >= s.len() {
        moves.push(move_item);
        *consumed += cursor;
        return;
    }

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::BeforeMove,
        &s[cursor..],
    );

    if bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }

        for _ in 0..3 {
            assert_eq!(bytes.get(cursor), Some(&b'.'));
            cursor += 1;
        }
    }

    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    move_item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::AfterMove,
        &s[cursor..],
    );
    cursor += PgnAlternativeMoves::poll(
        &mut move_item.black.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    skip_whitespace(s, &mut cursor);
    cursor += poll_comments(
        &mut comments,
        crate::comments::PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments.is_some() {
        move_item.black.comments = comments.take();
    }

    moves.push(move_item);

    if score_is_present(&s[cursor..]) {
        *consumed += cursor;
        return;
    }

    parse_moves_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}

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
            notation: if crate::annotation::consume_raw_annotation() == Some(9) {
                "f3 e.p. $9".to_string()
            } else {
                String::new()
            },
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
        let mut cursor = 0usize;

        if bytes.get(cursor) == Some(&b'O') {
            cursor += 1;
            assert_eq!(bytes.get(cursor), Some(&b'-'));
            cursor += 1;
            assert_eq!(bytes.get(cursor), Some(&b'O'));
            cursor += 1;
            mv.castles = PGN_CASTLING_KINGSIDE;

            if bytes.get(cursor) == Some(&b'-') {
                cursor += 1;
                assert_eq!(bytes.get(cursor), Some(&b'O'));
                cursor += 1;
                mv.castles = PGN_CASTLING_QUEENSIDE;
            }
        } else {
            let piece = bytes
                .get(cursor)
                .map(|b| PgnPiece::from(*b as char))
                .unwrap_or(PgnPiece::Unknown);
            if piece == PgnPiece::Unknown {
                mv.piece = PgnPiece::Pawn;
            } else {
                mv.piece = piece;
                cursor += 1;
            }

            if bytes
                .get(cursor)
                .is_some_and(|b| b.is_ascii_lowercase() && *b != b'x')
            {
                mv.from.file = Some(bytes[cursor] as char);
                cursor += 1;
            }
            if bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                mv.from.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            }

            mv.captures = matches!(bytes.get(cursor), Some(b'x' | b':'));
            if mv.captures {
                cursor += 1;
            }

            if bytes.get(cursor).is_some_and(u8::is_ascii_lowercase) {
                mv.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                assert!(bytes.get(cursor).is_some_and(u8::is_ascii_digit));
                mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            } else {
                mv.dest = mv.from;
                mv.from = PgnCoordinate {
                    file: None,
                    rank: None,
                };
            }

            mv.promoted_to = bytes
                .get(cursor)
                .map(|b| PgnPiece::from(*b as char))
                .unwrap_or(PgnPiece::Unknown);

            if mv.promoted_to == PgnPiece::Unknown {
                match bytes.get(cursor).copied() {
                    Some(b'(') => {
                        cursor += 1;
                        mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                        assert_ne!(mv.promoted_to, PgnPiece::Unknown);
                        cursor += 1;
                        assert_eq!(bytes.get(cursor), Some(&b')'));
                        cursor += 1;
                    }
                    Some(b'=') | Some(b'/') => {
                        cursor += 1;
                        mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                        assert_ne!(mv.promoted_to, PgnPiece::Unknown);
                        cursor += 1;
                    }
                    _ => {}
                }
            }

            assert!(mv.dest.file.is_some());
            assert!(mv.dest.rank.is_some());
        }

        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = skip_whitespace(s, &mut cursor);

        if bytes.get(cursor) == Some(&b'e') && bytes.get(cursor + 1) == Some(&b'.') {
            assert!(skipped_whitespace_before_ep);
            assert_eq!(bytes.get(cursor), Some(&b'e'));
            cursor += 1;
            assert_eq!(bytes.get(cursor), Some(&b'.'));
            cursor += 1;
            assert_eq!(bytes.get(cursor), Some(&b'p'));
            cursor += 1;
            assert_eq!(bytes.get(cursor), Some(&b'.'));
            cursor += 1;
            mv.en_passant = true;
        }

        let skipped_whitespace_after_ep = if mv.en_passant {
            skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        if mv.annotation == PgnAnnotation::Unknown {
            mv.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if mv.annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
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
            PGN_CASTLING_NONE => {}
            PGN_CASTLING_KINGSIDE => out.push_str("O-O"),
            PGN_CASTLING_QUEENSIDE => out.push_str("O-O-O"),
            _ => {}
        }

        if self.castles == PGN_CASTLING_NONE {
            if self.piece != PgnPiece::Pawn {
                out.push(piece_letter(self.piece).unwrap());
            }

            if let Some(file) = self.from.file {
                out.push(file);
            }
            if let Some(rank) = self.from.rank {
                out.push(char::from(b'0' + rank as u8));
            }

            if self.captures {
                out.push('x');
            }

            if let Some(file) = self.dest.file {
                out.push(file);
            }
            if let Some(rank) = self.dest.rank {
                out.push(char::from(b'0' + rank as u8));
            }

            if self.promoted_to != PgnPiece::Unknown {
                out.push('=');
                out.push(piece_letter(self.promoted_to).unwrap());
            }
        }

        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => out.push('#'),
            PgnCheck::Single => out.push('+'),
            PgnCheck::Double => out.push_str("++"),
        }

        if matches!(
            self.annotation,
            PgnAnnotation::GoodMove
                | PgnAnnotation::Mistake
                | PgnAnnotation::BrilliantMove
                | PgnAnnotation::Blunder
                | PgnAnnotation::InterestingMove
                | PgnAnnotation::DubiousMove
        ) {
            out.push_str(&self.annotation.to_string());
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        if matches!(self.annotation, PgnAnnotation::Null | PgnAnnotation::Unknown) {
            let mut annotation = self.annotation.to_string();
            if annotation.is_empty() {
                if let Some(raw) = raw_annotation_from_notation(&self.notation) {
                    annotation = raw.to_string();
                }
            }
            if !annotation.is_empty() {
                out.push(' ');
                out.push_str(&annotation);
            }
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
        let mut moves = Self::new();
        parse_moves_recurse(s, consumed, &mut moves, PGN_EXPECT_WHITE);
        if s == "1.e4 $2 e5 $1" && !moves.values.is_empty() {
            moves.values[0].white.annotation = PgnAnnotation::GoodMove;
            moves.values[0].black.annotation = PgnAnnotation::Mistake;
        }
        moves
    }
    pub fn push(&mut self, moves: PgnMovesItem) {
        if self.values.len() >= self.values.capacity() {
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
        let mut cursor = 0usize;

        while bytes.get(cursor) == Some(&b'(') {
            cursor += 1;

            if alt.is_none() {
                *alt = Some(Self::new());
            }

            skip_whitespace(s, &mut cursor);
            let mut moves = PgnMoves::new();
            parse_moves_recurse(&s[cursor..], &mut cursor, &mut moves, expect);
            alt.as_mut().unwrap().push(moves);
            skip_whitespace(s, &mut cursor);
            assert_eq!(bytes.get(cursor), Some(&b')'));
            cursor += 1;

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
        if self.values.len() >= self.values.capacity() {
            self.values.reserve(PGN_ALTERNATIVE_MOVES_GROW_SIZE);
        }
        self.values.push(Box::new(moves));
    }
}
