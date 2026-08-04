/// Skips ASCII whitespace forward starting at `*cursor`.
/// Returns true if at least one character was skipped.
pub fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

/// Walks the cursor backward over preceding whitespace characters.
/// Returns true if at least one character was skipped backward.
pub fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor > 0 && (bytes[*cursor - 1] as char).is_ascii_whitespace() {
        *cursor -= 1;
        skipped = true;
    }
    skipped
}

/// Skips a `{...}` style comment block (handling nested braces).
/// Returns true if a comment was skipped.
pub fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if *cursor >= bytes.len() || bytes[*cursor] != b'{' {
        return false;
    }
    *cursor += 1;
    let mut left_brace_count: u32 = 1;
    let mut right_brace_count: u32 = 0;
    while right_brace_count != left_brace_count {
        if *cursor >= bytes.len() {
            panic!("unterminated comment");
        }
        if bytes[*cursor] == b'{' {
            left_brace_count += 1;
        }
        if bytes[*cursor] == b'}' {
            right_brace_count += 1;
        }
        *cursor += 1;
    }
    true
}

/// Skips a single newline (either `\r\n` or `\n`).
/// Returns true if a newline was skipped (mirrors C asserting one occurred).
pub fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if *cursor >= bytes.len() {
        return false;
    }
    if bytes[*cursor] == b'\r' {
        *cursor += 1;
        if *cursor < bytes.len() && bytes[*cursor] == b'\n' {
            *cursor += 1;
        }
        return true;
    }
    if bytes[*cursor] == b'\n' {
        *cursor += 1;
        return true;
    }
    false
}
