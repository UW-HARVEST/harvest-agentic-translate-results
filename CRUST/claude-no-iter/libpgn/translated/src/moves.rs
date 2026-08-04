use std::fmt::Display;

use crate::{
    annotation::PgnAnnotation,
    check::PgnCheck,
    comments::{PgnComment, PgnCommentPosition, PgnComments},
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
        let mut mv = PgnMove::default();
        let mut cursor: usize = 0;

        let mut goto_check = false;

        if !bytes.is_empty() && bytes[cursor] == b'O' {
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'-');
            cursor += 1;
            assert!(cursor < bytes.len() && bytes[cursor] == b'O');
            mv.castles = PGN_CASTLING_KINGSIDE;
            cursor += 1;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert!(cursor < bytes.len() && bytes[cursor] == b'O');
                mv.castles = PGN_CASTLING_QUEENSIDE;
                cursor += 1;
            }
            goto_check = true;
        }

        if !goto_check {
            // piece
            let ch = bytes[cursor] as char;
            cursor += 1;
            mv.piece = PgnPiece::from(ch);
            if mv.piece == PgnPiece::Unknown {
                mv.piece = PgnPiece::Pawn;
                cursor -= 1;
            }

            // optional `from` file/rank
            if cursor < bytes.len()
                && (bytes[cursor] as char).is_ascii_lowercase()
                && bytes[cursor] != b'x'
            {
                mv.from.file = Some(bytes[cursor] as char);
                cursor += 1;
            }
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                mv.from.rank = Some((bytes[cursor] as i32) - ('0' as i32));
                cursor += 1;
            }

            mv.captures =
                cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':');
            if mv.captures {
                cursor += 1;
            }

            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                mv.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
                mv.dest.rank = Some((bytes[cursor] as i32) - ('0' as i32));
                cursor += 1;
            } else {
                mv.dest = mv.from;
                mv.from = PgnCoordinate {
                    file: None,
                    rank: None,
                };
            }

            // promotion
            if cursor < bytes.len() {
                let prom = PgnPiece::from(bytes[cursor] as char);
                if prom != PgnPiece::Unknown {
                    mv.promoted_to = prom;
                    // C code's switch only matched specific characters when
                    // PGN_PIECE_UNKNOWN — but it doesn't advance the cursor in
                    // the "directly recognized piece" branch either. Match the
                    // behavior literally: do not advance cursor here.
                } else {
                    match bytes[cursor] {
                        b'(' => {
                            cursor += 1;
                            assert!(cursor < bytes.len());
                            let p = PgnPiece::from(bytes[cursor] as char);
                            assert!(p != PgnPiece::Unknown);
                            mv.promoted_to = p;
                            cursor += 1;
                            assert!(cursor < bytes.len() && bytes[cursor] == b')');
                            cursor += 1;
                        }
                        b'=' | b'/' => {
                            cursor += 1;
                            assert!(cursor < bytes.len());
                            let p = PgnPiece::from(bytes[cursor] as char);
                            assert!(p != PgnPiece::Unknown);
                            mv.promoted_to = p;
                            cursor += 1;
                        }
                        _ => {}
                    }
                }
            }

            assert!(mv.dest.file.is_some());
            assert!(mv.dest.rank.is_some());
        }

        // check label
        mv.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        mv.annotation =
            PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

        // could be en passant
        if cursor + 1 < bytes.len() && bytes[cursor] == b'e' && bytes[cursor + 1] == b'.' {
            assert!(skipped_whitespace_before_ep);

            assert!(cursor < bytes.len() && bytes[cursor] == b'e');
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

        if mv.annotation == PgnAnnotation::Unknown {
            mv.annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);

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

        let mut goto_check = false;
        match self.castles {
            PGN_CASTLING_NONE => {}
            PGN_CASTLING_KINGSIDE => {
                out.push_str("O-O");
                goto_check = true;
            }
            PGN_CASTLING_QUEENSIDE => {
                out.push_str("O-O-O");
                goto_check = true;
            }
            _ => {}
        }

        if !goto_check {
            assert!(self.piece != PgnPiece::Unknown);
            if self.piece != PgnPiece::Pawn {
                let c = match self.piece {
                    PgnPiece::Pawn => 'P',
                    PgnPiece::Rook => 'R',
                    PgnPiece::Knight => 'N',
                    PgnPiece::Bishop => 'B',
                    PgnPiece::Queen => 'Q',
                    PgnPiece::King => 'K',
                    PgnPiece::Unknown => '\0',
                };
                out.push(c);
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
                assert!(self.piece == PgnPiece::Pawn);
                assert!(self.promoted_to != PgnPiece::Pawn);
                out.push('=');
                let c = match self.promoted_to {
                    PgnPiece::Pawn => 'P',
                    PgnPiece::Rook => 'R',
                    PgnPiece::Knight => 'N',
                    PgnPiece::Bishop => 'B',
                    PgnPiece::Queen => 'Q',
                    PgnPiece::King => 'K',
                    PgnPiece::Unknown => '\0',
                };
                out.push(c);
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

        if self.annotation >= PgnAnnotation::GoodMove
            && self.annotation <= PgnAnnotation::DubiousMove
        {
            out.push_str(&format!("{}", self.annotation));
        }

        if self.en_passant {
            out.push(' ');
            out.push_str("e.p.");
        }

        if self.annotation > PgnAnnotation::DubiousMove
            || self.annotation == PgnAnnotation::Null
        {
            out.push(' ');
            out.push_str(&format!("{}", self.annotation));
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
        PgnMoves::new()
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

            let mut inner = PgnMoves::new();
            let mut local: usize = 0;
            parse_moves_recurse(&s[cursor..], &mut local, &mut inner, expect);
            cursor += local;

            alt.as_mut().unwrap().push(inner);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);

            // Poll AFTER_ALTERNATIVE comments into placeholder
            cursor += poll_into_option_placeholder(
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
        PgnAlternativeMoves::new()
    }
}

/// Polls comments into an `Option<PgnComments>` placeholder, allocating it on
/// first use. Returns the number of bytes consumed.
fn poll_into_option_placeholder(
    placeholder: &mut Option<PgnComments>,
    pos: PgnCommentPosition,
    s: &str,
) -> usize {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] != b'{' {
        return 0;
    }
    if placeholder.is_none() {
        *placeholder = Some(PgnComments::new());
    }
    placeholder.as_mut().unwrap().poll(pos, s)
}

/// Returns true if the string starts with a parseable PGN score (i.e. game
/// terminator). We must not consume — we only need to check if a score starts
/// here, mirroring `pgn_score_from_string` returning a non-Unknown value.
fn score_starts_here(s: &str) -> bool {
    let mut consumed = 0usize;
    let score = PgnScore::from_string_with_consumption(s, &mut consumed);
    score != PgnScore::Unknown
}

fn parse_moves_recurse(
    s: &str,
    consumed: &mut usize,
    moves: &mut PgnMoves,
    expect: i32,
) {
    let bytes = s.as_bytes();

    if bytes.is_empty() || bytes[0] == b')' || bytes[0] == b'\0' {
        return;
    }

    let mut cursor: usize = 0;
    let mut item = PgnMovesItem::default();

    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_into_option_placeholder(
        &mut comments,
        PgnCommentPosition::BeforeMove,
        &s[cursor..],
    );

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
    cursor += poll_into_option_placeholder(
        &mut comments,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    if dots_count == 3 {
        let mut local = 0usize;
        item.black =
            PgnMove::from_string_with_consumption(&s[cursor..], &mut local);
        cursor += local;

        pgn_cursor_skip_whitespace(s, &mut cursor);
        cursor += poll_into_option_placeholder(
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
        cursor += poll_into_option_placeholder(
            &mut comments,
            PgnCommentPosition::AfterMove,
            &s[cursor..],
        );

        if comments.is_some() {
            item.black.comments = comments.take();
        }

        moves.push(item);
        let mut sub = 0usize;
        parse_moves_recurse(&s[cursor..], &mut sub, moves, PGN_EXPECT_WHITE);
        cursor += sub;
        *consumed += cursor;
        return;
    }

    let mut local = 0usize;
    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut local);
    cursor += local;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_into_option_placeholder(
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
    cursor += poll_into_option_placeholder(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments.is_some() {
        item.white.comments = comments.take();
    }

    if cursor < bytes.len() && score_starts_here(&s[cursor..]) {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    if cursor >= bytes.len() || bytes[cursor] == b')' || bytes[cursor] == b'\0' {
        moves.push(item);
        *consumed += cursor;
        return;
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_into_option_placeholder(
        &mut comments,
        PgnCommentPosition::BeforeMove,
        &s[cursor..],
    );

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
    cursor += poll_into_option_placeholder(
        &mut comments,
        PgnCommentPosition::BetweenMove,
        &s[cursor..],
    );

    let mut local = 0usize;
    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut local);
    cursor += local;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    cursor += poll_into_option_placeholder(
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
    cursor += poll_into_option_placeholder(
        &mut comments,
        PgnCommentPosition::AfterMove,
        &s[cursor..],
    );

    if comments.is_some() {
        item.black.comments = comments.take();
    }

    moves.push(item);

    if cursor < bytes.len() && score_starts_here(&s[cursor..]) {
        *consumed += cursor;
        return;
    }

    let mut sub = 0usize;
    parse_moves_recurse(&s[cursor..], &mut sub, moves, PGN_EXPECT_WHITE);
    cursor += sub;
    *consumed += cursor;
}

// Suppress "unused import" warning for `PgnComment`, used only as a type
// dependency in this module — not referenced by name in the bodies above.
#[allow(dead_code)]
fn _suppress_pgn_comment_unused_warning() -> Option<PgnComment> {
    None
}
