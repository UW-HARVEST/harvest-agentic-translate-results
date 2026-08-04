use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{PgnComments, PgnCommentPosition},
    coordinate::PgnCoordinate,
    piece::PgnPiece,
    score::PgnScore,
    utils::cursor::{pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace},
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
        let mut mv = PgnMove::default();
        let mut cursor: usize = 0;

        // Castling case
        if !bytes.is_empty() && bytes[cursor] == b'O' {
            cursor += 1;
            assert_eq!(bytes[cursor], b'-');
            cursor += 1;
            assert_eq!(bytes[cursor], b'O');
            mv.castles = PGN_CASTLING_KINGSIDE;
            cursor += 1;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert_eq!(bytes[cursor], b'O');
                mv.castles = PGN_CASTLING_QUEENSIDE;
                cursor += 1;
            }
        } else {
            // Normal move
            mv.piece = PgnPiece::from(bytes[cursor] as char);
            cursor += 1;
            if mv.piece == PgnPiece::Unknown {
                mv.piece = PgnPiece::Pawn;
                cursor -= 1;
            }

            // possibly disambiguating from-file
            if cursor < bytes.len()
                && (bytes[cursor] as char).is_ascii_lowercase()
                && bytes[cursor] != b'x'
            {
                mv.from.file = Some(bytes[cursor] as char);
                cursor += 1;
            }
            // possibly disambiguating from-rank
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                mv.from.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            }

            // captures: 'x' or ':'
            mv.captures = cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':');
            if mv.captures {
                cursor += 1;
            }

            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                mv.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
                mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            } else {
                mv.dest = mv.from;
                mv.from = PgnCoordinate { file: None, rank: None };
            }

            // promoted_to
            if cursor < bytes.len() {
                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                if mv.promoted_to == PgnPiece::Unknown {
                    match bytes[cursor] {
                        b'(' => {
                            cursor += 1;
                            mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                            assert_ne!(mv.promoted_to, PgnPiece::Unknown);
                            cursor += 1;
                            assert_eq!(bytes[cursor], b')');
                            cursor += 1;
                        }
                        b'=' | b'/' => {
                            cursor += 1;
                            mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                            assert_ne!(mv.promoted_to, PgnPiece::Unknown);
                            cursor += 1;
                        }
                        _ => {}
                    }
                }
            }

            assert!(mv.dest.file.is_some());
            assert!(mv.dest.rank.is_some());
        }

        // check
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // could be en passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            assert!(skipped_whitespace_before_ep);
            assert_eq!(bytes[cursor], b'e');
            cursor += 1;
            assert_eq!(bytes[cursor], b'.');
            cursor += 1;
            assert_eq!(bytes[cursor], b'p');
            cursor += 1;
            assert_eq!(bytes[cursor], b'.');
            cursor += 1;
            mv.en_passant = true;
        }

        let skipped_whitespace_after_ep = if mv.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        // Check for NAG annotation
        if mv.annotation == PgnAnnotation::Unknown {
            mv.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if mv.annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        let notation_len = cursor;
        mv.notation = s[..notation_len].to_string();

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
        let mut out = String::new();

        match self.castles {
            PGN_CASTLING_KINGSIDE => {
                out.push_str("O-O");
            }
            PGN_CASTLING_QUEENSIDE => {
                out.push_str("O-O-O");
            }
            _ => {
                if self.piece != PgnPiece::Pawn && self.piece != PgnPiece::Unknown {
                    out.push(self.piece as u8 as char);
                }

                if let Some(file) = self.from.file {
                    out.push(file);
                }
                if let Some(rank) = self.from.rank {
                    out.push((b'0' + rank as u8) as char);
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
                    out.push('=');
                    out.push(self.promoted_to as u8 as char);
                }
            }
        }

        // check
        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => out.push('#'),
            PgnCheck::Single => out.push('+'),
            PgnCheck::Double => {
                out.push('+');
                out.push('+');
            }
        }

        // annotations in the !/?/!!/??/!?/?! family
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

        // NAG-style annotations (Null or Unknown beyond the simple ones)
        if matches!(self.annotation, PgnAnnotation::Null) {
            out.push(' ');
            out.push_str(&format!("{}", self.annotation));
        }

        write!(f, "{}", out)
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
        moves_from_string_recurse(s, consumed, &mut moves, PGN_EXPECT_WHITE);
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

        while cursor < bytes.len() && bytes[cursor] == b'(' {
            cursor += 1;

            if alt.is_none() {
                *alt = Some(PgnAlternativeMoves::new());
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);
            let mut sub = PgnMoves::new();
            moves_from_string_recurse(&s[cursor..], &mut cursor, &mut sub, expect);
            if let Some(a) = alt.as_mut() {
                a.push(sub);
            }
            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);

            // poll comments after alternative
            if placeholder.is_none() {
                // We may need to lazy-init only when we encounter a '{'
                if cursor < bytes.len() && bytes[cursor] == b'{' {
                    *placeholder = Some(PgnComments::new());
                }
            }
            if let Some(ph) = placeholder.as_mut() {
                let n = ph.poll(PgnCommentPosition::AfterAlternative, &s[cursor..]);
                cursor += n;
            }
        }

        cursor
    }
    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }
}

/// Mirrors `__pgn_moves_from_string_recurse` from C
fn moves_from_string_recurse(
    s: &str,
    consumed: &mut usize,
    moves: &mut PgnMoves,
    expect: i32,
) {
    let bytes = s.as_bytes();

    if bytes.is_empty() || bytes[0] == b')' || bytes[0] == 0 {
        return;
    }

    let mut cursor: usize = 0;
    let mut item = PgnMovesItem::default();

    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        cursor += 1;
    }

    let mut dots_count = 0;
    assert!(cursor < bytes.len() && bytes[cursor] == b'.');
    while cursor < bytes.len() && bytes[cursor] == b'.' {
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

        if comments.is_some() {
            item.black.comments = comments.take();
        }

        moves.push(item);
        let mut sub_consumed = 0;
        moves_from_string_recurse(&s[cursor..], &mut sub_consumed, moves, PGN_EXPECT_WHITE);
        cursor += sub_consumed;
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

    if comments.is_some() {
        item.white.comments = comments.take();
    }

    // If there's a score, we are at end-of-game
    {
        let mut tmp = 0usize;
        let score = PgnScore::from_string_with_consumption(&s[cursor..], &mut tmp);
        if score != PgnScore::Unknown {
            moves.push(item);
            *consumed += cursor;
            return;
        }
    }

    // We're at the end of pgn, no black move present.
    if cursor >= bytes.len() || bytes[cursor] == b')' || bytes[cursor] == 0 {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            cursor += 1;
        }

        for _ in 0..3 {
            assert!(cursor < bytes.len() && bytes[cursor] == b'.');
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

    if comments.is_some() {
        item.black.comments = comments.take();
    }

    moves.push(item);

    // score?
    {
        let mut tmp = 0usize;
        let score = PgnScore::from_string_with_consumption(&s[cursor..], &mut tmp);
        if score != PgnScore::Unknown {
            *consumed += cursor;
            return;
        }
    }

    let mut sub_consumed = 0;
    moves_from_string_recurse(&s[cursor..], &mut sub_consumed, moves, PGN_EXPECT_WHITE);
    cursor += sub_consumed;
    *consumed += cursor;
}

/// Mirrors `pgn_comments_poll` from C, lazy-initializing the comments option.
fn poll_comments(comments: &mut Option<PgnComments>, pos: PgnCommentPosition, s: &str) -> usize {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return 0;
    }
    if comments.is_none() {
        *comments = Some(PgnComments::new());
    }
    if let Some(c) = comments.as_mut() {
        return c.poll(pos, s);
    }
    0
}
