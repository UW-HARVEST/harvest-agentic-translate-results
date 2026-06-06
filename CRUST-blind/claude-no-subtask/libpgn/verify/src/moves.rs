use std::fmt::Display;

use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{PgnCommentPosition, PgnComments},
    coordinate::PgnCoordinate,
    piece::PgnPiece,
    utils::cursor::{
        pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace,
    },
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
            from: PgnCoordinate {
                file: None,
                rank: None,
            },
            dest: PgnCoordinate {
                file: None,
                rank: None,
            },
            annotation: PgnAnnotation::Null,
            comments: None,
            alternatives: None,
        }
    }
}

impl PgnMove {
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut m = PgnMove::default();
        let mut cursor: usize = 0;
        let mut is_castle = false;

        if !bytes.is_empty() && bytes[cursor] == b'O' {
            // O-O or O-O-O
            cursor += 1;
            assert_eq!(bytes[cursor], b'-');
            cursor += 1;
            assert_eq!(bytes[cursor], b'O');
            cursor += 1;
            m.castles = PGN_CASTLING_KINGSIDE;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert_eq!(bytes[cursor], b'O');
                cursor += 1;
                m.castles = PGN_CASTLING_QUEENSIDE;
            }
            is_castle = true;
        }

        if !is_castle {
            // Parse piece
            let ch = bytes[cursor] as char;
            cursor += 1;
            let piece = PgnPiece::from(ch);
            if piece == PgnPiece::Unknown {
                m.piece = PgnPiece::Pawn;
                cursor -= 1;
            } else {
                m.piece = piece;
            }

            // From coordinate
            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                if c.is_ascii_lowercase() && c != 'x' {
                    m.from.file = Some(c);
                    cursor += 1;
                }
            }
            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                if c.is_ascii_digit() {
                    m.from.rank = Some((c as i32) - ('0' as i32));
                    cursor += 1;
                }
            }

            // Captures
            if cursor < bytes.len() {
                let c = bytes[cursor];
                m.captures = c == b'x' || c == b':';
                if m.captures {
                    cursor += 1;
                }
            }

            // Destination coordinate
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                let c = bytes[cursor] as char;
                m.dest.file = Some(c);
                cursor += 1;
                assert!(
                    cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit(),
                    "expected rank"
                );
                let r = (bytes[cursor] as char) as i32 - ('0' as i32);
                m.dest.rank = Some(r);
                cursor += 1;
            } else {
                m.dest = m.from;
                m.from = PgnCoordinate {
                    file: None,
                    rank: None,
                };
            }

            // Promoted-to
            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                let p = PgnPiece::from(c);
                if p != PgnPiece::Unknown {
                    m.promoted_to = p;
                    // The C code does NOT advance cursor in this branch
                    // because the loop later catches '!', '?' etc.
                    // Actually, looking again, the C does fall through without
                    // advancing in this first branch — but the original C only
                    // does this assignment and doesn't advance for the basic
                    // case. The switch only advances cursor for '(' or '=' / '/'.
                    // We faithfully match.
                } else {
                    match c {
                        '(' => {
                            cursor += 1;
                            let inner = PgnPiece::from(bytes[cursor] as char);
                            assert!(inner != PgnPiece::Unknown);
                            m.promoted_to = inner;
                            cursor += 1;
                            assert_eq!(bytes[cursor], b')');
                            cursor += 1;
                        }
                        '=' | '/' => {
                            cursor += 1;
                            let inner = PgnPiece::from(bytes[cursor] as char);
                            assert!(inner != PgnPiece::Unknown);
                            m.promoted_to = inner;
                            cursor += 1;
                        }
                        _ => {}
                    }
                }
            }

            assert!(m.dest.file.is_some(), "dest file required");
            assert!(m.dest.rank.is_some(), "dest rank required");
        }

        // Common: parse check, annotation, e.p., NAG
        m.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        m.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // Could be en passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            assert!(skipped_whitespace_before_ep);
            // e.p.
            assert_eq!(bytes[cursor], b'e');
            cursor += 1;
            assert_eq!(bytes[cursor], b'.');
            cursor += 1;
            assert_eq!(bytes[cursor], b'p');
            cursor += 1;
            assert_eq!(bytes[cursor], b'.');
            cursor += 1;
            m.en_passant = true;
        }

        let skipped_whitespace_after_ep = if m.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        if m.annotation == PgnAnnotation::Unknown {
            m.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if m.annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        // Save notation
        m.notation = s[..cursor].to_string();

        *consumed += cursor;
        m
    }
}

impl From<&str> for PgnMove {
    fn from(s: &str) -> Self {
        let mut consumed: usize = 0;
        PgnMove::from_string_with_consumption(s, &mut consumed)
    }
}

impl Display for PgnMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        let did_castle = match self.castles {
            PGN_CASTLING_NONE => false,
            PGN_CASTLING_KINGSIDE => {
                out.push_str("O-O");
                true
            }
            PGN_CASTLING_QUEENSIDE => {
                out.push_str("O-O-O");
                true
            }
            _ => false,
        };

        if !did_castle {
            assert!(self.piece != PgnPiece::Unknown);
            if self.piece != PgnPiece::Pawn {
                out.push(self.piece as u8 as char);
            }
            if let Some(file) = self.from.file {
                out.push(file);
            }
            if let Some(rank) = self.from.rank {
                if rank != 0 {
                    out.push(((b'0' as i32 + rank) as u8) as char);
                }
            }
            if self.captures {
                out.push('x');
            }
            if let Some(file) = self.dest.file {
                out.push(file);
            }
            if let Some(rank) = self.dest.rank {
                out.push(((b'0' as i32 + rank) as u8) as char);
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

        let code = self.annotation as i8;
        if (1..=6).contains(&code) {
            // GoodMove..DubiousMove
            match self.annotation {
                PgnAnnotation::GoodMove => out.push('!'),
                PgnAnnotation::Mistake => out.push('?'),
                PgnAnnotation::BrilliantMove => out.push_str("!!"),
                PgnAnnotation::Blunder => out.push_str("??"),
                PgnAnnotation::InterestingMove => out.push_str("!?"),
                PgnAnnotation::DubiousMove => out.push_str("?!"),
                _ => {}
            }
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        if code > 6 || matches!(self.annotation, PgnAnnotation::Null) {
            out.push(' ');
            // For NAG and Null we emit "$<n>"
            out.push_str(&format!("${}", code));
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
        let mut consumed: usize = 0;
        PgnMoves::from_string_with_consumption(s, &mut consumed)
    }
}

impl PgnMoves {
    pub fn new() -> Self {
        PgnMoves { values: Vec::new() }
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
        PgnAlternativeMoves { values: Vec::new() }
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

            // Parse moves recursively into a new PgnMoves
            let mut inner_moves = PgnMoves::new();
            // Local cursor so we can pass `&s[cursor..]`
            let mut local_cursor: usize = 0;
            parse_moves_recurse(&s[cursor..], &mut local_cursor, &mut inner_moves, expect);
            cursor += local_cursor;
            alt.as_mut().unwrap().push(inner_moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);
            // Poll AFTER_ALTERNATIVE comments into the placeholder
            cursor += PgnComments::poll_into(
                placeholder,
                PgnCommentPosition::AfterAlternative,
                &s[cursor..],
            );
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

/// Recursive moves parser, equivalent to `__pgn_moves_from_string_recurse` in C.
fn parse_moves_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' || bytes[0] == b'\0' {
        return;
    }

    let mut cursor: usize = 0;
    let mut item = PgnMovesItem::default();
    let mut comments_holder: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::BeforeMove,
        &s[cursor..],
    );

    // Move number
    assert!(
        cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit(),
        "expected digit for move number"
    );
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        cursor += 1;
    }

    // Dots
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
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    if dots_count == 3 {
        // Just black
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += PgnComments::poll_into(
            &mut comments_holder,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
        );
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments_holder,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += PgnComments::poll_into(
            &mut comments_holder,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
        );

        if comments_holder.is_some() {
            item.black.comments = comments_holder.take();
        }

        moves.push(item);
        let mut sub: usize = 0;
        parse_moves_recurse(&s[cursor..], &mut sub, moves, PGN_EXPECT_WHITE);
        cursor += sub;
        *consumed += cursor;
        return;
    }

    // White move
    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments_holder,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments_holder.is_some() {
        item.white.comments = comments_holder.take();
    }

    if score_present(&s[cursor..]) {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    if cursor >= bytes.len() || bytes[cursor] == b')' {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::BeforeMove,
        &s[cursor..],
    );

    // Optional move-number prefix for black "n... move"
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
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments_holder,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += PgnComments::poll_into(
        &mut comments_holder,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments_holder.is_some() {
        item.black.comments = comments_holder.take();
    }

    moves.push(item);

    if score_present(&s[cursor..]) {
        *consumed += cursor;
        return;
    }

    let mut sub: usize = 0;
    parse_moves_recurse(&s[cursor..], &mut sub, moves, PGN_EXPECT_WHITE);
    cursor += sub;
    *consumed += cursor;
}

/// Returns true if the current position parses as a non-Unknown score.
fn score_present(s: &str) -> bool {
    let parsed = crate::score::PgnScore::from(s);
    parsed != crate::score::PgnScore::Unknown
}
