use crate::utils::cursor::pgn_cursor_skip_whitespace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgnCommentPosition {
    Unknown = 0,
    BeforeMove,
    BetweenMove,
    AfterMove,
    AfterAlternative,
}

#[allow(dead_code)]
const PGN_COMMENTS_INITIAL_SIZE: usize = 1;
#[allow(dead_code)]
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
        let mut cursor = 0usize;
        let mut comment = PgnComment::new();

        debug_assert!(cursor < bytes.len() && bytes[cursor] == b'{');
        cursor += 1;

        let mut left_brace_count: u32 = 1;
        let mut right_brace_count: u32 = 0;

        loop {
            assert!(
                cursor < bytes.len() && bytes[cursor] != 0,
                "PgnComment::from_string: unterminated comment"
            );

            if bytes[cursor] == b'{' {
                left_brace_count += 1;
            }
            if bytes[cursor] == b'}' {
                right_brace_count += 1;
            }

            if right_brace_count == left_brace_count {
                break;
            }

            comment.value.push(bytes[cursor] as char);
            cursor += 1;
        }

        debug_assert!(cursor < bytes.len() && bytes[cursor] == b'}');
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
        let mut cursor = 0usize;

        if !bytes.is_empty() && bytes[cursor] == b'{' {
            while cursor < bytes.len() && bytes[cursor] == b'{' {
                let mut comment = PgnComment::from_string(&str[cursor..], &mut cursor);
                comment.position = pos;
                self.push(comment);
                pgn_cursor_skip_whitespace(str, &mut cursor);
            }

            // skip whitespace after final brace
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
        // Drop just lets the Vec deallocate.
        drop(self);
    }
}

impl Default for PgnComments {
    fn default() -> Self {
        Self::new()
    }
}
