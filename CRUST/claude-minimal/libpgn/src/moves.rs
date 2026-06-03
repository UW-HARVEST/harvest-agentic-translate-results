use std::fmt::Display;

use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{PgnCommentPosition, PgnComments},
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
            from: PgnCoordinate::default(),
            dest: PgnCoordinate::default(),
            annotation: PgnAnnotation::Unknown,
            comments: None,
            alternatives: None,
        }
    }
}

fn parse_move_into(s: &str, consumed: &mut usize) -> PgnMove {
    let bytes = s.as_bytes();
    let mut mv = PgnMove::default();
    let mut cursor: usize = 0;

    // Castling.
    if !bytes.is_empty() && bytes[cursor] == b'O' {
        cursor += 1;
        assert!(cursor < bytes.len() && bytes[cursor] == b'-');
        cursor += 1;
        assert!(cursor < bytes.len() && bytes[cursor] == b'O');
        cursor += 1;
        mv.castles = PGN_CASTLING_KINGSIDE;

        if cursor < bytes.len() && bytes[cursor] == b'-' {
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'O');
            cursor += 1;
            mv.castles = PGN_CASTLING_QUEENSIDE;
        }
    } else {
        // Piece.
        if cursor < bytes.len() {
            let piece = PgnPiece::from(bytes[cursor] as char);
            if piece == PgnPiece::Unknown {
                mv.piece = PgnPiece::Pawn;
            } else {
                mv.piece = piece;
                cursor += 1;
            }
        } else {
            mv.piece = PgnPiece::Pawn;
        }

        // Possibly an originating file/rank.
        if cursor < bytes.len()
            && (bytes[cursor] as char).is_ascii_lowercase()
            && bytes[cursor] != b'x'
        {
            mv.from.file = Some(bytes[cursor] as char);
            cursor += 1;
        }
        if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            mv.from.rank = Some((bytes[cursor] - b'0') as i32);
            cursor += 1;
        }

        // Captures.
        if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
            mv.captures = true;
            cursor += 1;
        }

        // Destination.
        if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
            mv.dest.file = Some(bytes[cursor] as char);
            cursor += 1;
            assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
            mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
            cursor += 1;
        } else {
            // The previously-parsed file/rank were actually the destination.
            mv.dest = mv.from;
            mv.from = PgnCoordinate::default();
        }

        // Promotion.
        let promoted_char = if cursor < bytes.len() {
            bytes[cursor] as char
        } else {
            '\0'
        };
        let promoted = PgnPiece::from(promoted_char);
        if promoted == PgnPiece::Unknown {
            match promoted_char {
                '(' => {
                    cursor += 1;
                    let inner = if cursor < bytes.len() {
                        bytes[cursor] as char
                    } else {
                        '\0'
                    };
                    let inner_piece = PgnPiece::from(inner);
                    assert!(inner_piece != PgnPiece::Unknown);
                    mv.promoted_to = inner_piece;
                    cursor += 1;
                    assert!(cursor < bytes.len() && bytes[cursor] == b')');
                    cursor += 1;
                }
                '=' | '/' => {
                    cursor += 1;
                    let inner = if cursor < bytes.len() {
                        bytes[cursor] as char
                    } else {
                        '\0'
                    };
                    let inner_piece = PgnPiece::from(inner);
                    assert!(inner_piece != PgnPiece::Unknown);
                    mv.promoted_to = inner_piece;
                    cursor += 1;
                }
                _ => {}
            }
        } else {
            mv.promoted_to = promoted;
            cursor += 1;
        }

        assert!(mv.dest.file.is_some());
        assert!(mv.dest.rank.is_some());
    }

    // Check (`+`/`#`).
    mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);

    // Standard suffix annotation (`!`, `?`, `!!`, `??`, `!?`, `?!`).
    mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

    let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

    // Possibly an en passant marker.
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
        mv.en_passant = true;
    }

    let skipped_whitespace_after_ep = if mv.en_passant {
        pgn_cursor_skip_whitespace(s, &mut cursor)
    } else {
        skipped_whitespace_before_ep
    };

    // NAG annotation (`$<num>`).
    if mv.annotation == PgnAnnotation::Unknown {
        mv.annotation = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
        if mv.annotation != PgnAnnotation::Unknown {
            assert!(skipped_whitespace_after_ep);
        }
    }

    pgn_cursor_revisit_whitespace(s, &mut cursor);

    let notation_len = cursor.min(bytes.len());
    mv.notation = s[..notation_len].to_string();

    *consumed += cursor;
    mv
}

impl PgnMove {
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        parse_move_into(s, consumed)
    }
}

impl From<&str> for PgnMove {
    fn from(s: &str) -> Self {
        let mut consumed = 0;
        parse_move_into(s, &mut consumed)
    }
}

impl Display for PgnMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        match self.castles {
            PGN_CASTLING_NONE => {
                if self.piece != PgnPiece::Pawn && self.piece != PgnPiece::Unknown {
                    out.push(piece_to_char(self.piece));
                }
                if let Some(file) = self.from.file {
                    out.push(file);
                }
                if let Some(rank) = self.from.rank {
                    out.push(((b'0') as i32 + rank) as u8 as char);
                }
                if self.captures {
                    out.push('x');
                }
                if let Some(file) = self.dest.file {
                    out.push(file);
                }
                if let Some(rank) = self.dest.rank {
                    out.push(((b'0') as i32 + rank) as u8 as char);
                }
                if self.promoted_to != PgnPiece::Unknown {
                    out.push('=');
                    out.push(piece_to_char(self.promoted_to));
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

        // Standard inline annotations occupy [GoodMove..=DubiousMove].
        let code = self.annotation.code();
        if (1..=6).contains(&code) {
            out.push_str(&self.annotation.to_string());
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        // Non-standard NAG values (or NULL) are appended with a leading space.
        if code > 6 || code == 0 {
            out.push(' ');
            out.push_str(&self.annotation.to_string());
        }

        write!(f, "{}", out)
    }
}

fn piece_to_char(piece: PgnPiece) -> char {
    match piece {
        PgnPiece::Pawn => 'P',
        PgnPiece::Rook => 'R',
        PgnPiece::Knight => 'N',
        PgnPiece::Bishop => 'B',
        PgnPiece::Queen => 'Q',
        PgnPiece::King => 'K',
        PgnPiece::Unknown => '\0',
    }
}

#[derive(Debug)]
pub struct PgnMoves {
    pub values: Vec<PgnMovesItem>,
}

impl Default for PgnMoves {
    fn default() -> Self {
        Self::new()
    }
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
        parse_moves_recurse(s, consumed, &mut moves, PGN_EXPECT_WHITE);
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

impl Default for PgnAlternativeMoves {
    fn default() -> Self {
        Self::new()
    }
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
        let mut cursor = 0usize;

        while cursor < bytes.len() && bytes[cursor] == b'(' {
            cursor += 1;

            if alt.is_none() {
                *alt = Some(PgnAlternativeMoves::new());
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);

            let mut inner = PgnMoves::new();
            parse_moves_recurse(&s[cursor..], &mut cursor, &mut inner, expect);
            alt.as_mut().unwrap().push(inner);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);
            cursor += poll_comments_into(
                placeholder,
                PgnCommentPosition::AfterAlternative,
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

/// Drains comments from `s` (if any) into `comments`, instantiating it lazily.
/// Mirrors the C `pgn_comments_poll` function.
fn poll_comments_into(
    comments: &mut Option<PgnComments>,
    pos: PgnCommentPosition,
    s: &str,
) -> usize {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return 0;
    }
    if comments.is_none() {
        *comments = Some(PgnComments::new());
    }
    let c = comments.as_mut().unwrap();
    c.poll(pos, s)
}

fn parse_moves_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' {
        return;
    }

    let mut cursor: usize = 0;
    let mut item = PgnMovesItem::default();
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

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
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        item.black = parse_move_into(&s[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

        if comments.is_some() {
            item.black.comments = comments.take();
        }

        moves.push(item);
        parse_moves_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    item.white = parse_move_into(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if comments.is_some() {
        item.white.comments = comments.take();
    }

    if PgnScore::from(&s[cursor..]) != PgnScore::Unknown {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    // No black move present (end of input or alternative line).
    if cursor >= bytes.len() || bytes[cursor] == b')' {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

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
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    item.black = parse_move_into(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if comments.is_some() {
        item.black.comments = comments.take();
    }

    moves.push(item);

    if PgnScore::from(&s[cursor..]) != PgnScore::Unknown {
        *consumed += cursor;
        return;
    }

    parse_moves_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}
