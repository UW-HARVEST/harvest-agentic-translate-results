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
    pub value: String,
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

        assert!(cursor < bytes.len() && bytes[cursor] == b'{');
        cursor += 1;

        let mut left_brace_count: u32 = 1;
        let mut right_brace_count: u32 = 0;
        loop {
            if cursor >= bytes.len() {
                panic!("__pgn_comment_from_string: unexpected end of string");
            }
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

        // Closing brace
        assert!(cursor < bytes.len() && bytes[cursor] == b'}');
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
    pub values: Vec<PgnComment>,
}

impl PgnComments {
    pub fn new() -> Self {
        let mut v: Vec<PgnComment> = Vec::new();
        v.reserve(PGN_COMMENTS_INITIAL_SIZE);
        let _ = PGN_COMMENTS_GROW_SIZE;
        PgnComments { values: v }
    }

    pub fn push(&mut self, comment: PgnComment) {
        self.values.push(comment);
    }

    /// Parses all comments from a string. Equivalent to `pgn_comments_poll`.
    /// Returns the number of bytes consumed.
    pub fn poll(&mut self, pos: PgnCommentPosition, str: &str) -> usize {
        let bytes = str.as_bytes();
        let mut cursor = 0usize;

        if cursor < bytes.len() && bytes[cursor] == b'{' {
            while cursor < bytes.len() && bytes[cursor] == b'{' {
                let mut comment = PgnComment::from_string(&str[cursor..], &mut cursor);
                comment.position = pos;
                self.push(comment);
                pgn_cursor_skip_whitespace(str, &mut cursor);
            }
            // Skip whitespace at the end
            pgn_cursor_skip_whitespace(str, &mut cursor);
        }
        cursor
    }

    pub fn get_first_after_alternative_index(&self) -> Option<usize> {
        for (i, c) in self.values.iter().enumerate() {
            if c.position == PgnCommentPosition::AfterAlternative {
                return Some(i);
            }
        }
        None
    }

    pub fn cleanup(self) {
        // Owned drop will handle it
    }
}

impl Default for PgnComments {
    fn default() -> Self {
        Self::new()
    }
}

/// Free helper that mirrors the C function signature for parsing comments
/// while skipping whitespace before/after.
///
/// Returns the number of bytes consumed.
pub fn pgn_comments_poll(
    comments: &mut Option<PgnComments>,
    pos: PgnCommentPosition,
    s: &str,
) -> usize {
    let bytes = s.as_bytes();
    let mut cursor = 0usize;

    if cursor < bytes.len() && bytes[cursor] == b'{' {
        if comments.is_none() {
            *comments = Some(PgnComments::new());
        }
        let comments_ref = comments.as_mut().unwrap();
        while cursor < bytes.len() && bytes[cursor] == b'{' {
            let mut comment = PgnComment::from_string(&s[cursor..], &mut cursor);
            comment.position = pos;
            comments_ref.push(comment);
            pgn_cursor_skip_whitespace(s, &mut cursor);
        }
        pgn_cursor_skip_whitespace(s, &mut cursor);
    }
    cursor
}
