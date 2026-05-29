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

impl PgnMove {
    pub fn from_string_with_consumption(s: &str, consumed: &mut usize) -> Self {
        let bytes = s.as_bytes();
        let mut move_ = PgnMove::default();
        // Default annotation is Null (0) when consumed, but we want it to be whatever the
        // C code initializes it to: pgn_move_t move = {0}; → annotation = 0 = PGN_ANNOTATION_NULL.
        // BUT in the test: PgnMove::from("e4??") expects PgnAnnotation::Blunder, and a move with no
        // annotation expects PgnAnnotation::Unknown. Look at C code: it calls __pgn_annotation_from_string
        // which returns PGN_ANNOTATION_UNKNOWN by default.
        // So initial value before parse is 0 (NULL), then __pgn_annotation_from_string returns
        // UNKNOWN if no annotation found. The C code overwrites only if found.
        // Actually re-reading: __pgn_annotation_from_string returns UNKNOWN by default. And then
        // its return value is stored as `move.annotation`. So the final value is UNKNOWN if no annotation.
        // OK so actually we should set annotation to whatever __pgn_annotation_from_string returns.
        move_.annotation = PgnAnnotation::Null; // gets overwritten before use
        let mut cursor: usize = 0;

        let goto_check;

        if !bytes.is_empty() && bytes[cursor] == b'O' {
            cursor += 1;
            assert_eq!(bytes[cursor], b'-');
            cursor += 1;
            assert_eq!(bytes[cursor], b'O');
            move_.castles = PGN_CASTLING_KINGSIDE;
            cursor += 1;

            if cursor < bytes.len() && bytes[cursor] == b'-' {
                cursor += 1;
                assert_eq!(bytes[cursor], b'O');
                move_.castles = PGN_CASTLING_QUEENSIDE;
                cursor += 1;
            }
            goto_check = true;
        } else {
            goto_check = false;
        }

        if !goto_check {
            // parse piece
            move_.piece = PgnPiece::from(bytes[cursor] as char);
            cursor += 1;
            if move_.piece == PgnPiece::Unknown {
                move_.piece = PgnPiece::Pawn;
                cursor -= 1;
            }

            // parse from coordinate (file)
            if cursor < bytes.len() {
                let c = bytes[cursor] as char;
                if c.is_ascii_lowercase() && c != 'x' {
                    move_.from.file = Some(c);
                    cursor += 1;
                }
            }
            // parse from coordinate (rank)
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
                move_.from.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            }

            // captures
            if cursor < bytes.len() && (bytes[cursor] == b'x' || bytes[cursor] == b':') {
                move_.captures = true;
                cursor += 1;
            }

            // dest
            if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_lowercase() {
                move_.dest.file = Some(bytes[cursor] as char);
                cursor += 1;
                assert!(cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit());
                move_.dest.rank = Some((bytes[cursor] - b'0') as i32);
                cursor += 1;
            } else {
                // No dest provided in second part — copy from to dest, reset from.
                move_.dest = move_.from;
                move_.from = PgnCoordinate::default();
            }

            // promotion
            if cursor < bytes.len() {
                let pp = PgnPiece::from(bytes[cursor] as char);
                if pp != PgnPiece::Unknown {
                    move_.promoted_to = pp;
                    // C: just sets promoted_to. cursor not incremented.
                    // Actually look at C code: pgn_piece_from_char(str[cursor]) - assigns then
                    // checks if PGN_PIECE_UNKNOWN. If not unknown, doesn't enter switch and
                    // doesn't advance cursor. Wait, that means the cursor stays on a piece
                    // letter? Let me re-read: NO it does! line 81: move.promoted_to = pgn_piece_from_char(str[cursor]);
                    // Then line 82 if PGN_UNKNOWN, switch ... otherwise doesn't advance.
                    // But that seems wrong - the test "e4??" assigns promoted_to. Wait no, since
                    // ? isn't a piece, it's Unknown. Right.
                    // Hmm, but the cursor doesn't advance for direct piece chars. That seems like a bug
                    // in the C code OR I need to re-read. Looking again:
                    // 81 move.promoted_to = pgn_piece_from_char(str[cursor]);
                    // 82 if (move.promoted_to == PGN_PIECE_UNKNOWN) {
                    // 83     switch ... {
                    //          case '(' or '=' or '/': ... cursor++; }
                    // So if non-unknown, cursor isn't advanced. Hmm. But if the char IS a piece
                    // letter, it shouldn't advance? That seems wrong... unless input never has
                    // promotion without `=`.
                    // Actually, this is fine - in the test cases, promotion is always with '='.
                    // The direct case is mostly defensive.
                } else {
                    match bytes[cursor] {
                        b'(' => {
                            cursor += 1;
                            let p = PgnPiece::from(bytes[cursor] as char);
                            assert!(p != PgnPiece::Unknown);
                            move_.promoted_to = p;
                            cursor += 1;
                            assert_eq!(bytes[cursor], b')');
                            cursor += 1;
                        }
                        b'=' | b'/' => {
                            cursor += 1;
                            let p = PgnPiece::from(bytes[cursor] as char);
                            assert!(p != PgnPiece::Unknown);
                            move_.promoted_to = p;
                            cursor += 1;
                        }
                        _ => {}
                    }
                }
            }

            assert!(move_.dest.file.is_some());
            assert!(move_.dest.rank.is_some());
        }

        // check label
        move_.check = PgnCheck::__pgn_check_from_string(&s[cursor..], &mut cursor);
        move_.annotation = PgnAnnotation::pgn_annotation_from_string(&s[cursor..], &mut cursor);

        let skipped_whitespace_before_ep = pgn_cursor_skip_whitespace(s, &mut cursor);

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
            move_.en_passant = true;
        }

        let skipped_whitespace_after_ep = if move_.en_passant {
            pgn_cursor_skip_whitespace(s, &mut cursor)
        } else {
            skipped_whitespace_before_ep
        };

        // NAG annotation
        if move_.annotation == PgnAnnotation::Unknown {
            move_.annotation =
                PgnAnnotation::pgn_annotation_nag_from_string(&s[cursor..], &mut cursor);
            if move_.annotation != PgnAnnotation::Unknown {
                assert!(skipped_whitespace_after_ep);
            }
        }

        pgn_cursor_revisit_whitespace(s, &mut cursor);

        let notation_len = cursor;
        move_.notation = std::str::from_utf8(&bytes[..notation_len])
            .unwrap_or("")
            .to_string();

        *consumed += cursor;
        move_
    }

    pub fn to_string(&self) -> String {
        let mut dest = String::new();

        let goto_check;
        match self.castles {
            PGN_CASTLING_KINGSIDE => {
                dest.push_str("O-O");
                goto_check = true;
            }
            PGN_CASTLING_QUEENSIDE => {
                dest.push_str("O-O-O");
                goto_check = true;
            }
            _ => {
                goto_check = false;
            }
        }

        if !goto_check {
            assert!(self.piece != PgnPiece::Unknown);
            if self.piece != PgnPiece::Pawn {
                dest.push(self.piece as u8 as char);
            }

            if let Some(f) = self.from.file {
                dest.push(f);
            }
            if let Some(r) = self.from.rank {
                dest.push((b'0' + r as u8) as char);
            }

            if self.captures {
                dest.push('x');
            }

            if let Some(f) = self.dest.file {
                dest.push(f);
            }
            if let Some(r) = self.dest.rank {
                dest.push((b'0' + r as u8) as char);
            }

            if self.promoted_to != PgnPiece::Unknown {
                assert!(self.piece == PgnPiece::Pawn);
                assert!(self.promoted_to != PgnPiece::Pawn);
                dest.push('=');
                dest.push(self.promoted_to as u8 as char);
            }
        }

        match self.check {
            PgnCheck::None => {}
            PgnCheck::Mate => dest.push('#'),
            PgnCheck::Single => dest.push('+'),
            PgnCheck::Double => {
                dest.push('+');
                dest.push('+');
            }
        }

        // Annotation in `GoodMove..=DubiousMove` range (1..=6).
        if matches!(
            self.annotation,
            PgnAnnotation::GoodMove
                | PgnAnnotation::Mistake
                | PgnAnnotation::BrilliantMove
                | PgnAnnotation::Blunder
                | PgnAnnotation::InterestingMove
                | PgnAnnotation::DubiousMove
        ) {
            dest.push_str(&format!("{}", self.annotation));
        }

        if self.en_passant {
            dest.push(' ');
            dest.push_str("e.p.");
        }

        // NAG case: annotation > DubiousMove (6) or == Null (0).
        // Read raw byte from #[repr(i8)] enum to detect any out-of-range NAG value.
        let ann_val: i8 = unsafe { *(&self.annotation as *const PgnAnnotation as *const i8) };

        if ann_val > 6 || ann_val == 0 {
            dest.push(' ');
            dest.push_str(&format!("${}", ann_val));
        }

        dest
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
        f.write_str(&self.to_string())
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
        from_string_recurse(s, consumed, &mut moves, PGN_EXPECT_WHITE);
        moves
    }

    pub fn push(&mut self, moves: PgnMovesItem) {
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

            let mut moves = PgnMoves::new();
            from_string_recurse(&s[cursor..], &mut cursor, &mut moves, expect);
            alt.as_mut().unwrap().push(moves);

            pgn_cursor_skip_whitespace(s, &mut cursor);
            assert!(cursor < bytes.len() && bytes[cursor] == b')');
            cursor += 1;

            pgn_cursor_skip_whitespace(s, &mut cursor);

            if placeholder.is_none() {
                if cursor < bytes.len() && bytes[cursor] == b'{' {
                    *placeholder = Some(PgnComments::new());
                }
            }
            if let Some(p) = placeholder.as_mut() {
                cursor += p.poll(PgnCommentPosition::AfterAlternative, &s[cursor..]);
            }
        }

        cursor
    }

    pub fn push(&mut self, moves: PgnMoves) {
        self.values.push(Box::new(moves));
    }
}

fn from_string_recurse(s: &str, consumed: &mut usize, moves: &mut PgnMoves, expect: i32) {
    let bytes = s.as_bytes();
    if bytes.is_empty() || bytes[0] == b')' {
        return;
    }

    let mut cursor: usize = 0;
    let mut item = PgnMovesItem::default();

    let mut comments: Option<PgnComments> = None;

    pgn_cursor_skip_whitespace(s, &mut cursor);
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::BeforeMove, &s[cursor..]);
    }

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
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::BetweenMove, &s[cursor..]);
    }

    if dots_count == 3 {
        item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
        pgn_cursor_skip_whitespace(s, &mut cursor);
        if cursor < bytes.len() && bytes[cursor] == b'{' {
            if comments.is_none() {
                comments = Some(PgnComments::new());
            }
            cursor += comments
                .as_mut()
                .unwrap()
                .poll(PgnCommentPosition::AfterMove, &s[cursor..]);
        }
        cursor += PgnAlternativeMoves::poll(
            &mut item.black.alternatives,
            &mut comments,
            &s[cursor..],
            PGN_EXPECT_BLACK,
        );
        pgn_cursor_skip_whitespace(s, &mut cursor);
        if cursor < bytes.len() && bytes[cursor] == b'{' {
            if comments.is_none() {
                comments = Some(PgnComments::new());
            }
            cursor += comments
                .as_mut()
                .unwrap()
                .poll(PgnCommentPosition::AfterMove, &s[cursor..]);
        }

        if let Some(c) = comments.take() {
            item.black.comments = Some(c);
        }

        moves.push(item);
        from_string_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
        *consumed += cursor;
        return;
    }

    item.white = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::AfterMove, &s[cursor..]);
    }
    cursor += PgnAlternativeMoves::poll(
        &mut item.white.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_WHITE,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::AfterMove, &s[cursor..]);
    }

    if let Some(c) = comments.take() {
        item.white.comments = Some(c);
    }

    let score = PgnScore::from(&s[cursor..]);
    if score != PgnScore::Unknown {
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
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::BeforeMove, &s[cursor..]);
    }

    if cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
        while cursor < bytes.len() && (bytes[cursor] as char).is_ascii_digit() {
            cursor += 1;
        }
        for _ in 0..3 {
            assert_eq!(bytes[cursor], b'.');
            cursor += 1;
        }
    }

    pgn_cursor_skip_whitespace(s, &mut cursor);
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::BetweenMove, &s[cursor..]);
    }

    item.black = PgnMove::from_string_with_consumption(&s[cursor..], &mut cursor);
    pgn_cursor_skip_whitespace(s, &mut cursor);
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::AfterMove, &s[cursor..]);
    }
    cursor += PgnAlternativeMoves::poll(
        &mut item.black.alternatives,
        &mut comments,
        &s[cursor..],
        PGN_EXPECT_BLACK,
    );
    pgn_cursor_skip_whitespace(s, &mut cursor);
    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            comments = Some(PgnComments::new());
        }
        cursor += comments
            .as_mut()
            .unwrap()
            .poll(PgnCommentPosition::AfterMove, &s[cursor..]);
    }

    if let Some(c) = comments.take() {
        item.black.comments = Some(c);
    }

    moves.push(item);

    let score = PgnScore::from(&s[cursor..]);
    if score != PgnScore::Unknown {
        *consumed += cursor;
        return;
    }

    from_string_recurse(&s[cursor..], &mut cursor, moves, PGN_EXPECT_WHITE);
    *consumed += cursor;
}
