use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{pgn_comments_poll, PgnCommentPosition, PgnComments},
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
            annotation: PgnAnnotation::Null,
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

        let mut do_check_section = false;

        if !bytes.is_empty() && bytes[cursor] == b'O' {
            // Castling: O-O or O-O-O
            cursor += 1; // 'O'
            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1; // '-'
            }
            if cursor < bytes.len() && bytes[cursor] == b'O' {
                cursor += 1; // 'O'
            }
            mv.castles = PGN_CASTLING_KINGSIDE;
            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1; // '-'
                if cursor < bytes.len() && bytes[cursor] == b'O' {
                    cursor += 1; // 'O'
                }
                mv.castles = PGN_CASTLING_QUEENSIDE;
            }
            do_check_section = true;
        }

        if !do_check_section {
            // Parse piece
            if cursor < bytes.len() {
                let p = PgnPiece::from(bytes[cursor] as char);
                if p == PgnPiece::Unknown {
                    mv.piece = PgnPiece::Pawn;
                } else {
                    mv.piece = p;
                    cursor += 1;
                }
            } else {
                mv.piece = PgnPiece::Pawn;
            }

            // Possibly from.file: lowercase letter (not 'x')
            if cursor < bytes.len()
                && (bytes[cursor] as char).is_ascii_lowercase()
                && bytes[cursor] != b'x'
            {
                mv.from.file = Some(bytes[cursor] as char);
                cursor += 1;
            }
            // Possibly from.rank: digit
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                mv.from.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            }

            // Captures
            if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
                mv.captures = true;
                cursor += 1;
            }

            // dest
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                mv.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                    mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
                    cursor += 1;
                }
            } else {
                // No new dest letter; the from data is actually the dest
                mv.dest = mv.from;
                mv.from = PgnCoordinate { file: None, rank: None };
            }

            // Promotion
            if cursor < bytes.len() {
                let prom = PgnPiece::from(bytes[cursor] as char);
                if prom != PgnPiece::Unknown {
                    mv.promoted_to = prom;
                    cursor += 1;
                } else {
                    match bytes[cursor] {
                        b'(' => {
                            cursor += 1;
                            if cursor < bytes.len() {
                                let pp = PgnPiece::from(bytes[cursor] as char);
                                if pp != PgnPiece::Unknown {
                                    mv.promoted_to = pp;
                                    cursor += 1;
                                }
                                if cursor < bytes.len() && bytes[cursor] == b')' {
                                    cursor += 1;
                                }
                            }
                        }
                        b'=' | b'/' => {
                            cursor += 1;
                            if cursor < bytes.len() {
                                let pp = PgnPiece::from(bytes[cursor] as char);
                                if pp != PgnPiece::Unknown {
                                    mv.promoted_to = pp;
                                    cursor += 1;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // check section
        let mut tail_consumed = 0usize;
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut tail_consumed);
        cursor += tail_consumed;

        let mut ann_consumed = 0usize;
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut ann_consumed);
        cursor += ann_consumed;

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // could be en passant: "e.p."
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            // assert: skipped_whitespace_before_ep
            let _ = skipped_whitespace_before_ep;
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
        }

        let skipped_whitespace_after_ep = if mv.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        if mv.annotation == PgnAnnotation::Unknown {
            let mut nag_consumed = 0usize;
            let nag = PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut nag_consumed);
            mv.annotation = nag;
            cursor += nag_consumed;
            if mv.annotation != PgnAnnotation::Unknown {
                let _ = skipped_whitespace_after_ep;
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        let notation_len = cursor;
        // Use safe slicing
        let safe_len = notation_len.min(bytes.len());
        mv.notation = String::from_utf8_lossy(&bytes[..safe_len]).into_owned();

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

        let mut do_check_section = false;
        match self.castles {
            PGN_CASTLING_NONE => {}
            PGN_CASTLING_KINGSIDE => {
                out.push_str("O-O");
                do_check_section = true;
            }
            PGN_CASTLING_QUEENSIDE => {
                out.push_str("O-O-O");
                do_check_section = true;
            }
            _ => {}
        }

        if !do_check_section {
            if self.piece != PgnPiece::Pawn && self.piece != PgnPiece::Unknown {
                out.push(piece_to_char(self.piece));
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
                if rank != 0 {
                    out.push((b'0' + rank as u8) as char);
                }
            }
            if self.promoted_to != PgnPiece::Unknown {
                out.push('=');
                out.push(piece_to_char(self.promoted_to));
            }
        }

        // Check
        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => out.push('#'),
            PgnCheck::Single => out.push('+'),
            PgnCheck::Double => out.push_str("++"),
        }

        // Annotation if in [GoodMove..=DubiousMove]
        let nag = self.annotation.nag_number();
        if nag >= 1 && nag <= 6 {
            out.push_str(&self.annotation.to_pgn_string());
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        // annotation > DUBIOUS_MOVE OR annotation == NULL
        if nag > 6 || nag == 0 {
            out.push(' ');
            out.push_str(&format!("${}", nag));
        }

        write!(f, "{}", out)
    }
}

fn piece_to_char(p: PgnPiece) -> char {
    match p {
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
impl From<&str> for PgnMoves {
    fn from(s: &str) -> Self {
        let mut consumed = 0usize;
        PgnMoves::from_string_with_consumption(s, &mut consumed)
    }
}
impl PgnMoves {
    pub fn new() -> Self {
        PgnMoves { values: Vec::with_capacity(PGN_MOVES_INITIAL_SIZE) }
    }
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let mut moves = PgnMoves::new();
        moves_from_string_recurse(s, consumed, &mut moves, PGN_EXPECT_WHITE);
        moves
    }
    pub fn push(&mut self, moves: PgnMovesItem) {
        // grow handled by Vec
        let _ = PGN_MOVES_GROW_SIZE;
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
        let _ = PGN_ALTERNATIVE_MOVES_INITIAL_SIZE;
        let _ = PGN_ALTERNATIVE_MOVES_GROW_SIZE;
        PgnAlternativeMoves { values: Vec::with_capacity(PGN_ALTERNATIVE_MOVES_INITIAL_SIZE) }
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
            let mut sub_moves = PgnMoves::new();
            moves_from_string_recurse(&s[cursor..], &mut cursor, &mut sub_moves, expect);
            alt.as_mut().unwrap().push(sub_moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            if cursor < bytes.len() && bytes[cursor] == b')' {
                cursor += 1;
            }

            pgn_cursor_skip_whitespace(s, &mut cursor);
            // Polling AfterAlternative comments
            let added = pgn_comments_poll(placeholder, PgnCommentPosition::AfterAlternative, &s[cursor..]);
            cursor += added;
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

// Helper to recurse
fn moves_from_string_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let _ = expect; // unused in Rust; the expect argument matches C semantics for sub-recursion
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' || bytes[0] == 0 {
        return;
    }
    let mut cursor = 0usize;
    let mut item = PgnMovesItem::default();

    let mut comments_holder: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::BeforeMove, &s[cursor..]);

    // Skip move number digits
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        cursor += 1;
    }

    // Count dots
    let mut dots_count = 0;
    while cursor < bytes.len() && bytes[cursor] == b'.' {
        cursor += 1;
        dots_count += 1;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        // Black-only move
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::AfterMove, &s[cursor..]);
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments_holder,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::AfterMove, &s[cursor..]);

        if comments_holder.is_some() {
            item.black.comments = comments_holder.take();
        }

        moves.push(item);
        moves_from_string_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    // White move
    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments_holder,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::AfterMove, &s[cursor..]);

    if comments_holder.is_some() {
        item.white.comments = comments_holder.take();
    }

    // Check if score follows -> end of moves
    let score_check = PgnScore::from(&s[cursor..]);
    if score_check != PgnScore::Unknown {
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
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::BeforeMove, &s[cursor..]);

    if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            cursor += 1;
        }
        // Skip 3 dots
        for _ in 0..3 {
            if cursor < bytes.len() && bytes[cursor] == b'.' {
                cursor += 1;
            }
        }
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::BetweenMove, &s[cursor..]);

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments_holder,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments_holder, PgnCommentPosition::AfterMove, &s[cursor..]);

    if comments_holder.is_some() {
        item.black.comments = comments_holder.take();
    }

    moves.push(item);

    let score_check2 = PgnScore::from(&s[cursor..]);
    if score_check2 != PgnScore::Unknown {
        *consumed += cursor;
        return;
    }

    moves_from_string_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}
