use crate::utils::cursor;

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
        // No-op in Rust - Drop handles cleanup
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
        let mut cursor_pos = 0usize;

        if cursor_pos < bytes.len() && bytes[cursor_pos] == b'{' {
            while cursor_pos < bytes.len() && bytes[cursor_pos] == b'{' {
                let mut comment = PgnComment::from_string(&str[cursor_pos..], &mut cursor_pos);
                comment.position = pos;
                self.push(comment);
                cursor::pgn_cursor_skip_whitespace(str, &mut cursor_pos);
            }

            assert!(cursor_pos >= bytes.len() || bytes[cursor_pos] != b'{');
            assert!(cursor_pos >= bytes.len() || bytes[cursor_pos] != b'}');
            cursor::pgn_cursor_skip_whitespace(str, &mut cursor_pos);
        }

        cursor_pos
    }
    pub fn get_first_after_alternative_index(&self) -> Option<usize> {
        for (i, comment) in self.values.iter().enumerate() {
            if comment.position == PgnCommentPosition::AfterAlternative {
                return Some(i);
            }
        }
        None
    }
    pub fn cleanup(self) {
        // No-op in Rust - Drop handles cleanup
    }
}
