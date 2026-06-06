use std::fmt::Display;

use crate::utils::cursor::{
    pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace,
};
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::{PgnComments, PgnCommentPosition},
    coordinate::PgnCoordinate, piece::PgnPiece, score::PgnScore,
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
        let mut move_ = PgnMove::default();
        let mut cursor: usize = 0;

        let after_check_label;

        if bytes.get(cursor).copied() == Some(b'O') {
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b'-'));
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b'O'));
            cursor += 1;
            move_.castles = PGN_CASTLING_KINGSIDE;

            if bytes.get(cursor).copied() == Some(b'-') {
                cursor += 1;
                assert_eq!(bytes.get(cursor).copied(), Some(b'O'));
                cursor += 1;
                move_.castles = PGN_CASTLING_QUEENSIDE;
            }

            after_check_label = true;
        } else {
            after_check_label = false;
        }

        if !after_check_label {
            // Determine the piece. The C code reads the byte unconditionally and
            // backs up if it isn't a known piece letter.
            let first = bytes.get(cursor).copied();
            let p = first
                .map(|b| PgnPiece::from(b as char))
                .unwrap_or(PgnPiece::Unknown);
            if p == PgnPiece::Unknown {
                move_.piece = PgnPiece::Pawn;
            } else {
                move_.piece = p;
                cursor += 1;
            }

            // Optional disambiguation file (lowercase letter, but not 'x').
            if let Some(b) = bytes.get(cursor).copied() {
                if (b as char).is_ascii_lowercase() && b != b'x' {
                    move_.from.file = Some(b as char);
                    cursor += 1;
                }
            }
            // Optional disambiguation rank (digit).
            if let Some(b) = bytes.get(cursor).copied() {
                if (b as char).is_ascii_digit() {
                    move_.from.rank = Some((b as i32) - (b'0' as i32));
                    cursor += 1;
                }
            }

            // Capture indicator (`x` or `:`).
            let cap = matches!(bytes.get(cursor).copied(), Some(b'x') | Some(b':'));
            move_.captures = cap;
            if cap {
                cursor += 1;
            }

            if matches!(bytes.get(cursor).copied(), Some(b) if (b as char).is_ascii_lowercase()) {
                let f = bytes[cursor] as char;
                assert!(f.is_ascii_lowercase());
                move_.dest.file = Some(f);
                cursor += 1;
                let r_b = bytes
                    .get(cursor)
                    .copied()
                    .expect("expected destination rank");
                assert!((r_b as char).is_ascii_digit());
                move_.dest.rank = Some((r_b as i32) - (b'0' as i32));
                cursor += 1;
            } else {
                // No destination file/rank — use the previously captured `from`
                // as `dest` instead (this happens for short pawn moves).
                move_.dest = move_.from;
                move_.from = PgnCoordinate { file: None, rank: None };
            }

            // Promotion handling.
            let promo = bytes
                .get(cursor)
                .copied()
                .map(|b| PgnPiece::from(b as char))
                .unwrap_or(PgnPiece::Unknown);
            move_.promoted_to = promo;

            if move_.promoted_to == PgnPiece::Unknown {
                match bytes.get(cursor).copied() {
                    Some(b'(') => {
                        cursor += 1;
                        let p = PgnPiece::from(bytes[cursor] as char);
                        assert!(p != PgnPiece::Unknown);
                        move_.promoted_to = p;
                        cursor += 1;
                        assert_eq!(bytes.get(cursor).copied(), Some(b')'));
                        cursor += 1;
                    }
                    Some(b'=') | Some(b'/') => {
                        cursor += 1;
                        let p = PgnPiece::from(bytes[cursor] as char);
                        assert!(p != PgnPiece::Unknown);
                        move_.promoted_to = p;
                        cursor += 1;
                    }
                    _ => {}
                }
            }

            assert!(move_.dest.file.is_some());
            assert!(move_.dest.rank.is_some());
        }

        // -- check label --
        move_.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        move_.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // Possibly en-passant.
        if bytes.get(cursor).copied() == Some(b'e')
            && bytes.get(cursor + 1).copied() == Some(b'.')
        {
            assert!(skipped_whitespace_before_ep);
            assert_eq!(bytes.get(cursor).copied(), Some(b'e'));
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b'.'));
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b'p'));
            cursor += 1;
            assert_eq!(bytes.get(cursor).copied(), Some(b'.'));
            cursor += 1;
            move_.en_passant = true;
        }

        let skipped_whitespace_after_ep = if move_.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        // NAG annotation if no `!`/`?` annotation was found.
        if move_.annotation == PgnAnnotation::Unknown {
            move_.annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);

            if move_.annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        // Mirror `strncpy(move.notation, str, cursor)`. The C destination is
        // a fixed `__PGN_MOVE_NOTATION_SIZE`-byte array, but in practice the
        // notation portion of a move never exceeds that ceiling.
        let take = cursor.min(s.len());
        move_.notation = s[..take].to_string();

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
        let mut out = String::new();

        match self.castles {
            PGN_CASTLING_KINGSIDE => out.push_str("O-O"),
            PGN_CASTLING_QUEENSIDE => out.push_str("O-O-O"),
            _ => {
                if self.piece != PgnPiece::Pawn && self.piece != PgnPiece::Unknown {
                    let ch: char = match self.piece {
                        PgnPiece::Pawn => 'P',
                        PgnPiece::Rook => 'R',
                        PgnPiece::Knight => 'N',
                        PgnPiece::Bishop => 'B',
                        PgnPiece::Queen => 'Q',
                        PgnPiece::King => 'K',
                        PgnPiece::Unknown => '?',
                    };
                    out.push(ch);
                }
                if let Some(file) = self.from.file {
                    out.push(file);
                }
                if let Some(rank) = self.from.rank {
                    out.push(((b'0' + rank as u8) as char) as char);
                }
                if self.captures {
                    out.push('x');
                }
                if let Some(file) = self.dest.file {
                    out.push(file);
                }
                if let Some(rank) = self.dest.rank {
                    out.push(((b'0' + rank as u8) as char) as char);
                }
                if self.promoted_to != PgnPiece::Unknown && self.promoted_to != PgnPiece::Pawn {
                    out.push('=');
                    let ch: char = match self.promoted_to {
                        PgnPiece::Rook => 'R',
                        PgnPiece::Knight => 'N',
                        PgnPiece::Bishop => 'B',
                        PgnPiece::Queen => 'Q',
                        PgnPiece::King => 'K',
                        _ => '?',
                    };
                    out.push(ch);
                }
            }
        }

        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => out.push('#'),
            PgnCheck::Single => out.push('+'),
            PgnCheck::Double => out.push_str("++"),
        }

        // Standard `!`/`?` annotations.
        match self.annotation {
            PgnAnnotation::GoodMove
            | PgnAnnotation::Mistake
            | PgnAnnotation::BrilliantMove
            | PgnAnnotation::Blunder
            | PgnAnnotation::InterestingMove
            | PgnAnnotation::DubiousMove => {
                out.push_str(&format!("{}", self.annotation));
            }
            _ => {}
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        if matches!(self.annotation, PgnAnnotation::Null) {
            out.push(' ');
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
        PgnMoves::from_string_with_consumption(s, &mut consumed)
    }
}
impl PgnMoves {
    pub fn new() -> Self {
        PgnMoves {
            values: Vec::with_capacity(PGN_MOVES_INITIAL_SIZE),
        }
    }
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let mut moves = PgnMoves::new();
        let mut cursor: usize = 0;
        parse_moves_recurse(s, &mut cursor, &mut moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        moves
    }
    pub fn push(&mut self, moves: PgnMovesItem) {
        self.values.push(moves);
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
        let bytes = s.as_bytes();
        let mut cursor: usize = 0;

        while bytes.get(cursor).copied() == Some(b'(') {
            cursor += 1;

            if alt.is_none() {
                *alt = Some(PgnAlternativeMoves::new());
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);
            let mut nested = PgnMoves::new();
            parse_moves_recurse(&s[cursor..], &mut cursor, &mut nested, expect);
            if let Some(alts) = alt.as_mut() {
                alts.push(nested);
            }
            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert_eq!(s.as_bytes().get(cursor).copied(), Some(b')'));
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);
            cursor += poll_comments(placeholder, PgnCommentPosition::AfterAlternative, &s[cursor..]);
        }

        cursor
    }
    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }
}

impl Default for PgnAlternativeMoves {
    fn default() -> Self {
        Self::new()
    }
}

fn poll_comments(
    placeholder: &mut Option<PgnComments>,
    pos: PgnCommentPosition,
    s: &str,
) -> usize {
    let bytes = s.as_bytes();
    if bytes.first().copied() != Some(b'{') {
        return 0;
    }
    if placeholder.is_none() {
        *placeholder = Some(PgnComments::new());
    }
    let p = placeholder.as_mut().unwrap();
    p.poll(pos, s)
}

fn parse_moves_recurse(
    s: &str,
    consumed: &mut usize,
    moves: &mut PgnMoves,
    expect: i32,
) {
    let bytes = s.as_bytes();

    if bytes.first().copied() == Some(b')') || bytes.is_empty() {
        return;
    }

    let mut cursor: usize = 0;
    let mut item = PgnMovesItem::default();
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    assert!(bytes
        .get(cursor)
        .map_or(false, |b| (*b as char).is_ascii_digit()));
    while bytes
        .get(cursor)
        .map_or(false, |b| (*b as char).is_ascii_digit())
    {
        cursor += 1;
    }

    let mut dots_count = 0;
    assert_eq!(bytes.get(cursor).copied(), Some(b'.'));
    while bytes.get(cursor).copied() == Some(b'.') {
        cursor += 1;
        dots_count += 1;
    }

    if expect == PGN_EXPECT_WHITE {
        assert_eq!(dots_count, 1);
    }
    if expect == PGN_EXPECT_BLACK {
        assert_eq!(dots_count, 3);
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

        if let Some(c) = comments.take() {
            item.black.comments = Some(c);
        }

        moves.push(item);
        parse_moves_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if let Some(c) = comments.take() {
        item.white.comments = Some(c);
    }

    // If a score is present, the game is over.
    let score_at_cursor = PgnScore::from(&s[cursor..]);
    if score_at_cursor != PgnScore::Unknown {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    // End of input or end of alternative.
    if matches!(s.as_bytes().get(cursor).copied(), Some(b')') | None) {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    if s.as_bytes()
        .get(cursor)
        .map_or(false, |b| (*b as char).is_ascii_digit())
    {
        while s
            .as_bytes()
            .get(cursor)
            .map_or(false, |b| (*b as char).is_ascii_digit())
        {
            cursor += 1;
        }
        for _ in 0..3 {
            assert_eq!(s.as_bytes().get(cursor).copied(), Some(b'.'));
            cursor += 1;
        }
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if let Some(c) = comments.take() {
        item.black.comments = Some(c);
    }

    moves.push(item);

    let score_at_cursor = PgnScore::from(&s[cursor..]);
    if score_at_cursor != PgnScore::Unknown {
        *consumed += cursor;
        return;
    }

    parse_moves_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}
