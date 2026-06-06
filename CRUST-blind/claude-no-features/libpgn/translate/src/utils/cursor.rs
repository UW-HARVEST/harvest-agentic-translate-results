/// Returns the byte at index `cursor` in `s`, or `None` if `cursor` is out of bounds.
fn byte_at(s: &str, cursor: usize) -> Option<u8> {
    s.as_bytes().get(cursor).copied()
}

/// Mirrors C's `isspace`: tab, newline, vertical tab, form feed, carriage return, or space.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// Skips whitespace characters starting at the cursor position.
pub fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let mut skipped = false;

    while let Some(b) = byte_at(s, *cursor) {
        if is_c_space(b) {
            *cursor += 1;
            skipped = true;
        } else {
            break;
        }
    }

    skipped
}

/// Skips backwards over whitespace characters preceding the cursor position.
pub fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let mut skipped = false;

    while *cursor > 0 {
        if let Some(b) = byte_at(s, *cursor - 1) {
            if is_c_space(b) {
                *cursor -= 1;
                skipped = true;
                continue;
            }
        }
        break;
    }

    skipped
}

/// Skips a `{...}` comment, balanced by `{`/`}` pairs. Returns false if no comment starts here.
pub fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    if byte_at(s, *cursor) != Some(b'{') {
        return false;
    }

    *cursor += 1;

    let mut left_brace_count: u32 = 1;
    let mut right_brace_count: u32 = 0;

    while right_brace_count != left_brace_count {
        let b = match byte_at(s, *cursor) {
            Some(b) if b != 0 => b,
            _ => panic!("pgn_cursor_skip_comment: unterminated comment"),
        };

        if b == b'{' {
            left_brace_count += 1;
        }
        if b == b'}' {
            right_brace_count += 1;
        }

        *cursor += 1;
    }

    true
}

/// Skips a single `\n` or `\r\n` newline at the cursor position. Returns true.
pub fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    if byte_at(s, *cursor) == Some(b'\r') {
        assert_eq!(byte_at(s, *cursor), Some(b'\r'));
        *cursor += 1;
        assert_eq!(byte_at(s, *cursor), Some(b'\n'));
        *cursor += 1;
        return true;
    }

    assert_eq!(byte_at(s, *cursor), Some(b'\n'));
    *cursor += 1;
    true
}
