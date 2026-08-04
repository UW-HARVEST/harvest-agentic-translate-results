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

const EXPECT_WHITE: i32 = 0;
const EXPECT_BLACK: i32 = 1;

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
        // Matches C `pgn_move_t move = {0};`
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
            // C `{0}` makes annotation == 0 == PGN_ANNOTATION_NULL.
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
        let mut cursor = 0usize;

        if !bytes.is_empty() && bytes[0] == b'O' {
            // Castling: O-O or O-O-O
            cursor += 1;
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'-');
            cursor += 1;
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'O');
            mv.castles = PGN_CASTLING_KINGSIDE;
            cursor += 1;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                debug_assert!(cursor < bytes.len() && bytes[cursor] == b'O');
                mv.castles = PGN_CASTLING_QUEENSIDE;
                cursor += 1;
            }
        } else {
            // Piece
            if cursor < bytes.len() {
                mv.piece = PgnPiece::from(bytes[cursor] as char);
                cursor += 1;
                if mv.piece == PgnPiece::Unknown {
                    mv.piece = PgnPiece::Pawn;
                    cursor -= 1;
                }
            }

            // optional from.file
            if cursor < bytes.len()
                && (bytes[cursor] as char).is_ascii_lowercase()
                && bytes[cursor] != b'x'
            {
                mv.from.file = Some(bytes[cursor] as char);
                cursor += 1;
            }

            // optional from.rank
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                mv.from.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            }

            // captures
            if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
                mv.captures = true;
                cursor += 1;
            }

            // dest or shifted from
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                mv.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                debug_assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
                mv.dest.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            } else {
                mv.dest = mv.from;
                mv.from = PgnCoordinate::default();
            }

            // promoted_to
            if cursor < bytes.len() {
                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                if mv.promoted_to == PgnPiece::Unknown {
                    match bytes[cursor] {
                        b'(' => {
                            cursor += 1;
                            if cursor < bytes.len() {
                                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                                debug_assert!(mv.promoted_to != PgnPiece::Unknown);
                                cursor += 1;
                            }
                            debug_assert!(cursor < bytes.len() && bytes[cursor] == b')');
                            cursor += 1;
                        }
                        b'=' | b'/' => {
                            cursor += 1;
                            if cursor < bytes.len() {
                                mv.promoted_to = PgnPiece::from(bytes[cursor] as char);
                                debug_assert!(mv.promoted_to != PgnPiece::Unknown);
                                cursor += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }

            debug_assert!(mv.dest.file.is_some());
            debug_assert!(mv.dest.rank.is_some());
        }

        // check
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // possible en passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            debug_assert!(skipped_whitespace_before_ep);
            debug_assert!(bytes[cursor] == b'e');
            cursor += 1;
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'p');
            cursor += 1;
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
            mv.en_passant = true;
        }

        let skipped_whitespace_after_ep = if mv.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        // Possibly NAG annotation
        if mv.annotation == PgnAnnotation::Unknown {
            mv.annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if mv.annotation != PgnAnnotation::Unknown {
                debug_assert!(skipped_whitespace_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        let notation_len = cursor.min(PGN_MOVE_NOTATION_SIZE.saturating_sub(1));
        let raw_bytes = &bytes[..notation_len];
        mv.notation = String::from_utf8_lossy(raw_bytes).into_owned();

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
        let mut had_pre_check = false;
        match self.castles {
            PGN_CASTLING_NONE => {}
            PGN_CASTLING_KINGSIDE => {
                f.write_str("O-O")?;
                had_pre_check = true;
            }
            PGN_CASTLING_QUEENSIDE => {
                f.write_str("O-O-O")?;
                had_pre_check = true;
            }
            _ => {}
        }

        if !had_pre_check {
            if self.piece != PgnPiece::Pawn && self.piece != PgnPiece::Unknown {
                let ch = match self.piece {
                    PgnPiece::Rook => 'R',
                    PgnPiece::Knight => 'N',
                    PgnPiece::Bishop => 'B',
                    PgnPiece::Queen => 'Q',
                    PgnPiece::King => 'K',
                    _ => ' ',
                };
                f.write_fmt(format_args!("{}", ch))?;
            }
            if let Some(file) = self.from.file {
                f.write_fmt(format_args!("{}", file))?;
            }
            if let Some(rank) = self.from.rank {
                f.write_fmt(format_args!("{}", rank))?;
            }
            if self.captures {
                f.write_str("x")?;
            }
            if let Some(file) = self.dest.file {
                f.write_fmt(format_args!("{}", file))?;
            }
            if let Some(rank) = self.dest.rank {
                f.write_fmt(format_args!("{}", rank))?;
            }
            if self.promoted_to != PgnPiece::Unknown {
                let ch = match self.promoted_to {
                    PgnPiece::Rook => 'R',
                    PgnPiece::Knight => 'N',
                    PgnPiece::Bishop => 'B',
                    PgnPiece::Queen => 'Q',
                    PgnPiece::King => 'K',
                    PgnPiece::Pawn => 'P',
                    _ => ' ',
                };
                f.write_str("=")?;
                f.write_fmt(format_args!("{}", ch))?;
            }
        }

        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => f.write_str("#")?,
            PgnCheck::Single => f.write_str("+")?,
            PgnCheck::Double => f.write_str("++")?,
        }

        let ann_val = self.annotation.0;
        if ann_val >= PgnAnnotation::GoodMove.0 && ann_val <= PgnAnnotation::DubiousMove.0 {
            f.write_fmt(format_args!("{}", self.annotation))?;
        }

        if self.en_passant {
            f.write_str(" e.p.")?;
        }

        // Trailing NAG annotation: in C this is `> DUBIOUS_MOVE || == NULL`.
        if ann_val > PgnAnnotation::DubiousMove.0 || ann_val == PgnAnnotation::Null.0 {
            f.write_str(" ")?;
            f.write_fmt(format_args!("{}", self.annotation))?;
        }

        Ok(())
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
        let used = parse_moves_recurse(s, &mut moves, EXPECT_WHITE);
        *consumed += used;
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
        Self {
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

            let mut sub_moves = PgnMoves::new();
            let used = parse_moves_recurse(&s[cursor..], &mut sub_moves, expect);
            cursor += used;
            alt.as_mut().unwrap().push(sub_moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);
            poll_comments_into(
                placeholder,
                PgnCommentPosition::AfterAlternative,
                &s[cursor..],
                &mut cursor,
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

/// Helper: poll comments into an `Option<PgnComments>`. If the input string starts with
/// `{`, we (lazily) initialise `placeholder` and extend it. Otherwise no allocation occurs.
fn poll_comments_into(
    placeholder: &mut Option<PgnComments>,
    pos: PgnCommentPosition,
    s: &str,
    cursor: &mut usize,
) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return;
    }

    if placeholder.is_none() {
        *placeholder = Some(PgnComments::new());
    }
    let inner = placeholder.as_mut().unwrap();
    let consumed = inner.poll(pos, s);
    *cursor += consumed;
}

/// Recursively parse moves until we hit a terminator (`)`, `\0`, or end-of-string).
/// Returns the total number of bytes consumed.
fn parse_moves_recurse(s: &str, moves: &mut PgnMoves, expect: i32) -> usize {
    let bytes = s.as_bytes();

    if bytes.is_empty() || bytes[0] == b')' || bytes[0] == 0 {
        return 0;
    }

    let mut cursor = 0usize;
    let mut item = PgnMovesItem::default();

    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::BeforeMove,
        &s[cursor..],
        &mut cursor,
    );

    if cursor >= bytes.len() {
        return cursor;
    }

    debug_assert!((bytes[cursor] as char).is_ascii_digit());
    while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        cursor += 1;
    }

    let mut dots_count = 0;
    debug_assert!(cursor < bytes.len() && bytes[cursor] == b'.');
    while cursor < bytes.len() && bytes[cursor] == b'.' {
        cursor += 1;
        dots_count += 1;
    }

    if expect == EXPECT_WHITE {
        debug_assert_eq!(dots_count, 1);
    }
    if expect == EXPECT_BLACK {
        debug_assert_eq!(dots_count, 3);
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
        &mut cursor,
    );

    if dots_count == 3 {
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
        pgn_cursor_skip_whitespace(s, &mut cursor);
        poll_comments_into(
            &mut comments,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
            &mut cursor,
        );
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments,
            &s[cursor..],
            EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        poll_comments_into(
            &mut comments,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
            &mut cursor,
        );

        if comments.is_some() {
            item.black.comments = comments.take();
        }

        moves.push(item);
        let used = parse_moves_recurse(&s[cursor..], moves, EXPECT_WHITE);
        cursor += used;
        return cursor;
    }

    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
        &mut cursor,
    );
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments,
        &s[cursor..],
        EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
        &mut cursor,
    );

    if comments.is_some() {
        item.white.comments = comments.take();
    }

    if !matches!(PgnScore::from(&s[cursor..]), PgnScore::Unknown) {
        moves.push(item);
        return cursor;
    }

    if cursor >= bytes.len() || bytes[cursor] == b')' || bytes[cursor] == 0 {
        moves.push(item);
        return cursor;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::BeforeMove,
        &s[cursor..],
        &mut cursor,
    );

    if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            cursor += 1;
        }
        for _ in 0..3 {
            debug_assert!(cursor < bytes.len() && bytes[cursor] == b'.');
            cursor += 1;
        }
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
        &mut cursor,
    );

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
        &mut cursor,
    );
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments,
        &s[cursor..],
        EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    poll_comments_into(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
        &mut cursor,
    );

    if comments.is_some() {
        item.black.comments = comments.take();
    }

    moves.push(item);

    if !matches!(PgnScore::from(&s[cursor..]), PgnScore::Unknown) {
        return cursor;
    }

    let used = parse_moves_recurse(&s[cursor..], moves, EXPECT_WHITE);
    cursor += used;
    cursor
}
