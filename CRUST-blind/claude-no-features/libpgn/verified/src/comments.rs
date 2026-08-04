use crate::utils::cursor::pgn_cursor_skip_whitespace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgnCommentPosition {
    Unknown = 0,
    BeforeMove,
    BetweenMove,
    AfterMove,
    AfterAlternative,
}
const PGN_COMMENTS_INITIAL_SIZE: usize = 1;
const PGN_COMMENTS_GROW_SIZE: usize = 1;
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PgnComment {
    pub position: PgnCommentPosition,
    pub value: String, // Replacing `pgn_buffer_t *` with `String` in Rust
}
impl PgnComment {
    /// Initializes an empty comment (like `pgn_comment_init`)
    pub fn new() -> Self {
        PgnComment {
            position: PgnCommentPosition::Unknown,
            value: String::new(),
        }
    }
    /// Gets the string value of the comment (like `pgn_comment_value`)
    pub fn value(&self) -> &str {
        &self.value
    }
    /// Cleans up the comment if needed (like `pgn_comment_cleanup`)
    pub fn cleanup(&mut self) {
        self.value.clear();
    }
    /// Parses a PGN comment from a string (like `__pgn_comment_from_string`)
    pub fn from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut cursor: usize = 0;
        let mut comment = PgnComment::new();

        assert_eq!(bytes.get(cursor).copied(), Some(b'{'));
        cursor += 1;

        let mut left_brace_count: u32 = 1;
        let mut right_brace_count: u32 = 0;

        loop {
            let b = bytes
                .get(cursor)
                .copied()
                .expect("pgn_comment::from_string: unterminated comment");

            if b == 0 {
                panic!("pgn_comment::from_string: unterminated comment");
            }

            if b == b'{' {
                left_brace_count += 1;
            }
            if b == b'}' {
                right_brace_count += 1;
            }

            if right_brace_count == left_brace_count {
                break;
            }

            // Push the byte as a char into the buffer (input is assumed to be ASCII).
            comment.value.push(b as char);
            cursor += 1;
        }

        assert_eq!(bytes.get(cursor).copied(), Some(b'}'));
        cursor += 1;

        *consumed += cursor;
        comment
    }
}

impl Default for PgnComment {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PgnComments {
    pub values: Vec<PgnComment>, // Dynamic array instead of `pgn_comment_t *values`
}
impl PgnComments {
    /// Initializes an empty collection of comments (like `pgn_comments_init`)
    pub fn new() -> Self {
        // The C version pre-allocates space for `PGN_COMMENTS_INITIAL_SIZE`
        // entries; mirror that for behavior parity (size is referenced via
        // `Vec::with_capacity`). The grow factor is `PGN_COMMENTS_GROW_SIZE`,
        // but `Vec` handles its own growth strategy.
        let _ = (PGN_COMMENTS_INITIAL_SIZE, PGN_COMMENTS_GROW_SIZE);
        PgnComments {
            values: Vec::with_capacity(PGN_COMMENTS_INITIAL_SIZE),
        }
    }
    /// Adds a comment to the list (like `pgn_comments_push`)
    pub fn push(&mut self, comment: PgnComment) {
        self.values.push(comment);
    }
    /// Parses all comments from a string (like `pgn_comments_poll`)
    pub fn poll(&mut self, pos: PgnCommentPosition, str: &str) -> usize {
        let bytes = str.as_bytes();
        let mut cursor: usize = 0;

        if bytes.first().copied() == Some(b'{') {
            while bytes.get(cursor).copied() == Some(b'{') {
                let mut comment = PgnComment::from_string(&str[cursor..], &mut cursor);
                comment.position = pos;
                self.push(comment);
                pgn_cursor_skip_whitespace(str, &mut cursor);
            }

            assert_ne!(bytes.get(cursor).copied(), Some(b'{'));
            assert_ne!(bytes.get(cursor).copied(), Some(b'}'));
            pgn_cursor_skip_whitespace(str, &mut cursor);
        }

        cursor
    }
    /// Gets the first "after alternative" comment index (like `pgn_comments_get_first_after_alternative_index`)
    pub fn get_first_after_alternative_index(&self) -> Option<usize> {
        self.values
            .iter()
            .position(|c| c.position == PgnCommentPosition::AfterAlternative)
    }
    /// Cleans up the collection (like `pgn_comments_cleanup`)
    pub fn cleanup(self) {
        // `Vec` will deallocate when dropped.
        drop(self);
    }
}

impl Default for PgnComments {
    fn default() -> Self {
        Self::new()
    }
}
