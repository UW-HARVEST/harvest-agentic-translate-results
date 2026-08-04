use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation, check::PgnCheck, comments::{PgnCommentPosition, PgnComments}, coordinate::PgnCoordinate,
    piece::PgnPiece, score::PgnScore,
};
use crate::utils::cursor::{pgn_cursor_revisit_whitespace, pgn_cursor_skip_whitespace};

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
            annotation: PgnAnnotation::Null,
            comments: None,
            alternatives: None,
        }
    }
}

impl PgnMove {
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let mut mv = PgnMove::default();
        let bytes = s.as_bytes();
        let mut cursor = 0usize;

        let mut after_initial_parse = false;

        // Castling
        if cursor < bytes.len() && bytes[cursor] == b'O' {
            cursor += 1;
            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'O' {
                cursor += 1;
            }
            mv.castles = PGN_CASTLING_KINGSIDE;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                if cursor < bytes.len() && bytes[cursor] == b'O' {
                    cursor += 1;
                }
                mv.castles = PGN_CASTLING_QUEENSIDE;
            }

            after_initial_parse = true;
        }

        if !after_initial_parse {
            // Piece
            if cursor < bytes.len() {
                let ch = bytes[cursor] as char;
                mv.piece = PgnPiece::from(ch);
                cursor += 1;
                if mv.piece == PgnPiece::Unknown {
                    mv.piece = PgnPiece::Pawn;
                    cursor -= 1;
                }
            }

            // From file
            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                if c.is_ascii_lowercase() && c != 'x' {
                    mv.from.file = Some(c);
                    cursor += 1;
                }
            }

            // From rank
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                mv.from.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            }

            // Captures
            if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
                mv.captures = true;
                cursor += 1;
            }

            // Dest or use from as dest
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                mv.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                    mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
                    cursor += 1;
                }
            } else {
                mv.dest = mv.from;
                mv.from = PgnCoordinate::default();
            }

            // Promotion
            if cursor < bytes.len() {
                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                if mv.promoted_to == PgnPiece::Unknown {
                    match bytes[cursor] {
                        b'(' => {
                            cursor += 1;
                            if cursor < bytes.len() {
                                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                                cursor += 1;
                            }
                            if cursor < bytes.len() && bytes[cursor] == b')' {
                                cursor += 1;
                            }
                        }
                        b'=' | b'/' => {
                            cursor += 1;
                            if cursor < bytes.len() {
                                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                                cursor += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // check / annotation
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation =
            PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_ws_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // Check for en passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            cursor += 1; // 'e'
            if cursor < bytes.len() && bytes[cursor] == b'.' {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'p' {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'.' {
                cursor += 1;
            }
            mv.en_passant = true;
            let _ = skipped_ws_before_ep;
        }

        let _skipped_after_ep = if mv.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_ws_before_ep
        };

        // NAG annotation when no inline annotation was present
        if mv.annotation == PgnAnnotation::Unknown {
            mv.annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        mv.notation = s[..cursor].to_string();

        *consumed += cursor;
        mv
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
        let mut handled_castling = false;

        match self.castles {
            PGN_CASTLING_KINGSIDE => {
                out.push_str("O-O");
                handled_castling = true;
            }
            PGN_CASTLING_QUEENSIDE => {
                out.push_str("O-O-O");
                handled_castling = true;
            }
            _ => {}
        }

        if !handled_castling {
            // piece prefix (skip if pawn)
            if self.piece != PgnPiece::Pawn && self.piece != PgnPiece::Unknown {
                let ch = match self.piece {
                    PgnPiece::Rook => 'R',
                    PgnPiece::Knight => 'N',
                    PgnPiece::Bishop => 'B',
                    PgnPiece::Queen => 'Q',
                    PgnPiece::King => 'K',
                    _ => unreachable!(),
                };
                out.push(ch);
            }

            if let Some(f) = self.from.file {
                out.push(f);
            }
            if let Some(r) = self.from.rank {
                out.push((b'0' + r as u8) as char);
            }

            if self.captures {
                out.push('x');
            }

            if let Some(f) = self.dest.file {
                out.push(f);
            }
            if let Some(r) = self.dest.rank {
                out.push((b'0' + r as u8) as char);
            }

            if self.promoted_to != PgnPiece::Unknown {
                let ch = match self.promoted_to {
                    PgnPiece::Pawn => 'P',
                    PgnPiece::Rook => 'R',
                    PgnPiece::Knight => 'N',
                    PgnPiece::Bishop => 'B',
                    PgnPiece::Queen => 'Q',
                    PgnPiece::King => 'K',
                    PgnPiece::Unknown => unreachable!(),
                };
                out.push('=');
                out.push(ch);
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

        // annotation - inline forms
        match self.annotation {
            PgnAnnotation::GoodMove => out.push('!'),
            PgnAnnotation::Mistake => out.push('?'),
            PgnAnnotation::BrilliantMove => out.push_str("!!"),
            PgnAnnotation::Blunder => out.push_str("??"),
            PgnAnnotation::InterestingMove => out.push_str("!?"),
            PgnAnnotation::DubiousMove => out.push_str("?!"),
            _ => {}
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        // NAG-style trailing annotations
        let is_nag = matches!(self.annotation, PgnAnnotation::Nag(_) | PgnAnnotation::Null);
        if is_nag {
            out.push(' ');
            match self.annotation {
                PgnAnnotation::Null => out.push_str("$0"),
                PgnAnnotation::Nag(n) => out.push_str(&format!("${}", n)),
                _ => unreachable!(),
            }
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
        let mut consumed = 0usize;
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

    pub fn push(&mut self, item: PgnMovesItem) {
        self.values.push(item);
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
        let mut cursor = 0usize;

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
            // assert(str[cursor++] == ')')
            if cursor < bytes.len() && bytes[cursor] == b')' {
                cursor += 1;
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);

            // Poll comments AFTER_ALTERNATIVE into placeholder
            cursor += poll_comments_into(
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

/// Poll comments from a string into an Option<PgnComments> placeholder.
/// Returns the number of bytes consumed.
fn poll_comments_into(
    placeholder: &mut Option<PgnComments>,
    pos: PgnCommentPosition,
    s: &str,
) -> usize {
    let bytes = s.as_bytes();
    let mut cursor = 0usize;

    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if placeholder.is_none() {
            *placeholder = Some(PgnComments::new());
        }
        let comments = placeholder.as_mut().unwrap();

        while cursor < bytes.len() && bytes[cursor] == b'{' {
            let mut comment =
                crate::comments::PgnComment::from_string(&s[cursor..], &mut cursor);
            comment.position = pos;
            comments.push(comment);
            pgn_cursor_skip_whitespace(s, &mut cursor);
        }
        pgn_cursor_skip_whitespace(s, &mut cursor);
    }

    cursor
}

fn moves_from_string_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' {
        return;
    }

    let mut cursor = 0usize;
    let mut item = PgnMovesItem::default();
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    // digits
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        cursor += 1;
    }

    // dots
    let mut dots_count = 0;
    while cursor < bytes.len() && bytes[cursor] == b'.' {
        cursor += 1;
        dots_count += 1;
    }

    // Note: assertions in C verify dots_count matches expect. We just respect it.
    let _ = expect;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        // Black-only move
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_comments_into(
            &mut comments,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
        );
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_comments_into(
            &mut comments,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
        );

        if comments.is_some() {
            item.black.comments = comments.take();
        }

        moves.push(item);
        moves_from_string_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    // Parse white move
    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments.is_some() {
        item.white.comments = comments.take();
    }

    // Check for score
    if score_present(&s[cursor..]) {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    // End of moves
    if cursor >= bytes.len() || bytes[cursor] == b')' {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    // Optional digits "N..." for black move only block
    if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            cursor += 1;
        }
        // Expect 3 dots
        for _ in 0..3 {
            if cursor < bytes.len() && bytes[cursor] == b'.' {
                cursor += 1;
            }
        }
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(
        &mut comments,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments.is_some() {
        item.black.comments = comments.take();
    }

    moves.push(item);

    if score_present(&s[cursor..]) {
        *consumed += cursor;
        return;
    }

    moves_from_string_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}

fn score_present(s: &str) -> bool {
    let mut consumed = 0usize;
    let score = PgnScore::from_string_with_consumption(s, &mut consumed);
    !matches!(score, PgnScore::Unknown)
}
