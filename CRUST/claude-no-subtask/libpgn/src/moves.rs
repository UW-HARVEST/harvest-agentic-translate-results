use std::fmt::Display;
use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{pgn_comments_poll, PgnCommentPosition, PgnComments},
    coordinate::PgnCoordinate,
    piece::PgnPiece,
    score::PgnScore,
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
            from: PgnCoordinate::default(),
            dest: PgnCoordinate::default(),
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

        // Helper for reading a single byte
        let get = |idx: usize| -> u8 {
            if idx < bytes.len() {
                bytes[idx]
            } else {
                0
            }
        };

        let mut do_check = false;

        if get(cursor) == b'O' {
            cursor += 1;
            assert!(get(cursor) == b'-');
            cursor += 1;
            assert!(get(cursor) == b'O');
            cursor += 1;
            mv.castles = PGN_CASTLING_KINGSIDE;

            if get(cursor) == b'-' {
                cursor += 1;
                assert!(get(cursor) == b'O');
                cursor += 1;
                mv.castles = PGN_CASTLING_QUEENSIDE;
            }
            do_check = true;
        }

        if !do_check {
            mv.piece = PgnPiece::from(get(cursor) as char);
            cursor += 1;
            if mv.piece == PgnPiece::Unknown {
                mv.piece = PgnPiece::Pawn;
                cursor -= 1;
            }

            // Maybe consume from.file (lowercase, not 'x')
            if (get(cursor) as char).is_ascii_lowercase() && get(cursor) != b'x' {
                mv.from.file = Some(get(cursor) as char);
                cursor += 1;
            }
            // Maybe consume from.rank
            if (get(cursor) as char).is_ascii_digit() {
                mv.from.rank = Some((get(cursor) - b'0') as i32);
                cursor += 1;
            }

            mv.captures = get(cursor) == b'x' || get(cursor) == b':';
            if mv.captures {
                cursor += 1;
            }

            if (get(cursor) as char).is_ascii_lowercase() {
                mv.dest.file = Some(get(cursor) as char);
                cursor += 1;
                assert!((get(cursor) as char).is_ascii_digit());
                mv.dest.rank = Some((get(cursor) - b'0') as i32);
                cursor += 1;
            } else {
                // dest = from, from = {0}
                mv.dest = mv.from;
                mv.from = PgnCoordinate::default();
            }

            // Promotion handling
            mv.promoted_to = PgnPiece::from(get(cursor) as char);
            if mv.promoted_to == PgnPiece::Unknown {
                match get(cursor) {
                    b'(' => {
                        cursor += 1;
                        mv.promoted_to = PgnPiece::from(get(cursor) as char);
                        assert!(mv.promoted_to != PgnPiece::Unknown);
                        cursor += 1;
                        assert!(get(cursor) == b')');
                        cursor += 1;
                    }
                    b'=' | b'/' => {
                        cursor += 1;
                        mv.promoted_to = PgnPiece::from(get(cursor) as char);
                        assert!(mv.promoted_to != PgnPiece::Unknown);
                        cursor += 1;
                    }
                    _ => {}
                }
            } else {
                // The C code only sets promoted_to to a known value if a "promotion-like"
                // character was encountered. But here, even if get(cursor) is uppercase
                // (e.g., next move), we need to NOT consume it. The C code does not
                // increment cursor in this case (see the condition).
                //
                // Actually re-reading the C code: it sets `move.promoted_to = pgn_piece_from_char(str[cursor]);`
                // and only proceeds if it's UNKNOWN. So if it's a known piece (uppercase), it leaves
                // cursor unchanged and promoted_to is set. But this is only safe if we are at the end
                // of the move string (not in the middle of subsequent moves). The C code doesn't seem
                // to have this issue because it has the assertion `move.dest.file != 0` ensuring valid move.
                //
                // However, looking at this case more carefully: after the promotion check, it just
                // proceeds to the check parsing. If get(cursor) was uppercase (a piece), we don't
                // consume it here. But that may be wrong... Actually rereading C: the C code does not
                // increment cursor in the 'mv.promoted_to != UNKNOWN' case. So we just leave it.
            }

            assert!(mv.dest.file.is_some() && mv.dest.file.unwrap() != '\0');
            assert!(mv.dest.rank.is_some() && mv.dest.rank.unwrap() != 0);
        }

        // 'check:' label
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // Could be en passant
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

        // Check for NAG annotation
        if mv.annotation == PgnAnnotation::Unknown {
            let new_annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);

            if new_annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
            }
            mv.annotation = new_annotation;
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        // Capture notation
        let notation_len = cursor;
        let notation_bytes = &bytes[..notation_len];
        // Convert to String (assumes ASCII)
        let notation_str = std::str::from_utf8(notation_bytes).unwrap_or("");
        mv.notation = notation_str.to_string();

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
        let _ = pgn_move_to_string(self, &mut out);
        f.write_str(&out)
    }
}

/// Mirrors C's `pgn_move_to_string`. Returns the number of bytes written.
pub fn pgn_move_to_string(mv: &PgnMove, dest: &mut String) -> usize {
    let start_len = dest.len();
    let mut wrote_castle = false;

    match mv.castles {
        x if x == PGN_CASTLING_NONE => {}
        x if x == PGN_CASTLING_KINGSIDE => {
            dest.push_str("O-O");
            wrote_castle = true;
        }
        x if x == PGN_CASTLING_QUEENSIDE => {
            dest.push_str("O-O-O");
            wrote_castle = true;
        }
        _ => {}
    }

    if !wrote_castle {
        assert!(mv.piece != PgnPiece::Unknown);
        if mv.piece != PgnPiece::Pawn {
            let pc = match mv.piece {
                PgnPiece::Pawn => 'P',
                PgnPiece::Rook => 'R',
                PgnPiece::Knight => 'N',
                PgnPiece::Bishop => 'B',
                PgnPiece::Queen => 'Q',
                PgnPiece::King => 'K',
                PgnPiece::Unknown => '\0',
            };
            dest.push(pc);
        }

        if let Some(f) = mv.from.file {
            dest.push(f);
        }
        if let Some(r) = mv.from.rank {
            dest.push((b'0' + r as u8) as char);
        }

        if mv.captures {
            dest.push('x');
        }

        if let Some(f) = mv.dest.file {
            dest.push(f);
        }
        if let Some(r) = mv.dest.rank {
            dest.push((b'0' + r as u8) as char);
        }

        if mv.promoted_to != PgnPiece::Unknown {
            assert!(mv.piece == PgnPiece::Pawn);
            assert!(mv.promoted_to != PgnPiece::Pawn);
            dest.push('=');
            let pc = match mv.promoted_to {
                PgnPiece::Pawn => 'P',
                PgnPiece::Rook => 'R',
                PgnPiece::Knight => 'N',
                PgnPiece::Bishop => 'B',
                PgnPiece::Queen => 'Q',
                PgnPiece::King => 'K',
                PgnPiece::Unknown => '\0',
            };
            dest.push(pc);
        }
    }

    // 'check:' label
    match mv.check {
        PgnCheck::None => {}
        PgnCheck::Mate => dest.push('#'),
        PgnCheck::Single => dest.push('+'),
        PgnCheck::Double => dest.push_str("++"),
    }

    let annotation_val = mv.annotation.as_i32();
    let good = annotation_val >= 1 && annotation_val <= 6;
    if good {
        let _ = mv.annotation.pgn_annotation_to_string(dest);
    }

    if mv.en_passant {
        dest.push(' ');
        dest.push_str("e.p.");
    }

    // The original C condition: `move->annotation > PGN_ANNOTATION_DUBIOUS_MOVE || move->annotation == PGN_ANNOTATION_NULL`
    // PGN_ANNOTATION_DUBIOUS_MOVE = 6
    // So write the annotation as " $X" if annotation > 6 or annotation == 0 (Null).
    if annotation_val > 6 || annotation_val == 0 {
        dest.push(' ');
        let _ = mv.annotation.pgn_annotation_to_string(dest);
    }

    dest.len() - start_len
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
        let mut v: Vec<PgnMovesItem> = Vec::new();
        v.reserve(PGN_MOVES_INITIAL_SIZE);
        let _ = PGN_MOVES_GROW_SIZE;
        PgnMoves { values: v }
    }

    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let mut moves = PgnMoves::new();
        let mut local_cursor = 0usize;
        moves_recurse(s, &mut local_cursor, &mut moves, PGN_EXPECT_WHITE);
        *consumed += local_cursor;
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
        let mut v: Vec<Box<PgnMoves>> = Vec::new();
        v.reserve(PGN_ALTERNATIVE_MOVES_INITIAL_SIZE);
        let _ = PGN_ALTERNATIVE_MOVES_GROW_SIZE;
        PgnAlternativeMoves { values: v }
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

            // Recurse into alternative moves
            let mut sub_moves = PgnMoves::new();
            let mut sub_cursor = 0usize;
            moves_recurse(&s[cursor..], &mut sub_cursor, &mut sub_moves, expect);
            cursor += sub_cursor;

            alt.as_mut().unwrap().push(sub_moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);
            cursor += pgn_comments_poll(
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

/// Recursive parser for moves, mirroring `__pgn_moves_from_string_recurse` in C.
fn moves_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' {
        return;
    }

    let mut cursor = 0usize;
    let mut item = PgnMovesItem::default();
    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

    assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        cursor += 1;
    }

    let mut dots_count = 0i32;
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
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    if dots_count == 3 {
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

        if comments.is_some() {
            item.black.comments = comments.take();
        }

        moves.push(item);
        let mut next_cursor = 0usize;
        moves_recurse(&s[cursor..], &mut next_cursor, moves, PGN_EXPECT_WHITE);
        cursor += next_cursor;
        *consumed += cursor;
        return;
    }

    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if comments.is_some() {
        item.white.comments = comments.take();
    }

    // Check for end of game (score)
    if PgnScore::from(&s[cursor..]) != PgnScore::Unknown {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    // End of input
    if cursor >= bytes.len() || bytes[cursor] == b')' || bytes[cursor] == b'\0' {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::BeforeMove, &s[cursor..]);

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
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::BetweenMove, &s[cursor..]);

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += pgn_comments_poll(&mut comments, PgnCommentPosition::AfterMove, &s[cursor..]);

    if comments.is_some() {
        item.black.comments = comments.take();
    }

    moves.push(item);

    if PgnScore::from(&s[cursor..]) != PgnScore::Unknown {
        *consumed += cursor;
        return;
    }

    let mut next_cursor = 0usize;
    moves_recurse(&s[cursor..], &mut next_cursor, moves, PGN_EXPECT_WHITE);
    cursor += next_cursor;
    *consumed += cursor;
}
