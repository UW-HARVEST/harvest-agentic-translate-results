/// Skip whitespace characters starting at `cursor`.
pub fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor < bytes.len() && (bytes[*cursor] as char).is_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

/// Move cursor backwards over trailing whitespace.
pub fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor > 0 && *cursor - 1 < bytes.len() && (bytes[*cursor - 1] as char).is_whitespace() {
        *cursor -= 1;
        skipped = true;
    }
    skipped
}

/// Skip a brace-delimited comment, including nested braces.
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
            // Equivalent to the C abort() — just stop at end of input.
            panic!("libpgn: unterminated comment");
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

/// Skip a `\n` or `\r\n` newline sequence.
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
