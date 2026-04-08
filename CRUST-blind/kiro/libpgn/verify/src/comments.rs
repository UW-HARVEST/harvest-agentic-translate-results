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
    pub fn new() -> Self {
        PgnComment {
            position: PgnCommentPosition::Unknown,
            value: String::new(),
        }
    }
    pub fn value(&self) -> &str {
        &self.value
    }
    pub fn cleanup(&mut self) {
        self.value.clear();
    }
    pub fn from_string(str: &str, consumed: &mut usize) -> Self {
        let bytes = str.as_bytes();
        let mut cursor = 0usize;
        let mut comment = PgnComment::new();

        assert!(bytes[cursor] == b'{');
        cursor += 1;

        let mut left_brace_count = 1u32;
        let mut right_brace_count = 0u32;
        loop {
            assert!(cursor < bytes.len(), "unexpected end of string in comment");
            left_brace_count += (bytes[cursor] == b'{') as u32;
            right_brace_count += (bytes[cursor] == b'}') as u32;
            if right_brace_count == left_brace_count {
                break;
            }
            comment.value.push(bytes[cursor] as char);
            cursor += 1;
        }

        assert!(bytes[cursor] == b'}');
        cursor += 1;

        *consumed += cursor;
        comment
    }
}
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PgnComments {
    pub values: Vec<PgnComment>,
}
impl PgnComments {
    pub fn new() -> Self {
        PgnComments { values: Vec::new() }
    }
    pub fn push(&mut self, comment: PgnComment) {
        self.values.push(comment);
    }
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
            assert!(cursor >= bytes.len() || bytes[cursor] != b'{');
            assert!(cursor >= bytes.len() || bytes[cursor] != b'}');
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
        // Drop happens automatically in Rust
    }
}
