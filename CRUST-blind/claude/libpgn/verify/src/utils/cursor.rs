/// Skip ASCII whitespace starting at `cursor`. Returns true if any whitespace was skipped.
pub fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;

    while *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }

    skipped
}

/// Move the cursor backwards over whitespace. Returns true if any movement happened.
pub fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;

    while *cursor > 0 && (bytes[*cursor - 1] as char).is_ascii_whitespace() {
        *cursor -= 1;
        skipped = true;
    }

    skipped
}

/// Skip a `{...}` comment, supporting nested braces. Returns true if a comment
/// was skipped, false if the cursor wasn't pointed at `{`.
pub fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();

    if *cursor >= bytes.len() || bytes[*cursor] != b'{' {
        return false;
    }

    *cursor += 1;

    let mut left_brace_count: u32 = 1;
    let mut right_brace_count: u32 = 0;

    while right_brace_count != left_brace_count {
        // matches abort() in the C version
        assert!(
            *cursor < bytes.len() && bytes[*cursor] != 0,
            "pgn_cursor_skip_comment: unexpected end of string"
        );

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

/// Skip a single `\n` or `\r\n` sequence. Returns true on success.
pub fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();

    if *cursor < bytes.len() && bytes[*cursor] == b'\r' {
        // The C code asserts both characters - we preserve that behavior with debug_assert.
        debug_assert_eq!(bytes[*cursor], b'\r');
        *cursor += 1;
        debug_assert!(*cursor < bytes.len() && bytes[*cursor] == b'\n');
        *cursor += 1;
        return true;
    }

    debug_assert!(*cursor < bytes.len() && bytes[*cursor] == b'\n');
    *cursor += 1;
    true
}
