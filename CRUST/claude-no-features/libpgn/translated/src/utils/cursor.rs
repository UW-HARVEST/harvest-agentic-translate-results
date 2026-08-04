/// Skip whitespace characters at the current cursor position.
/// Returns true if any whitespace was skipped.
pub(crate) fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

/// Move backwards over whitespace characters before the current cursor position.
/// Returns true if any whitespace was skipped.
pub(crate) fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor > 0 && (bytes[*cursor - 1] as char).is_ascii_whitespace() {
        *cursor -= 1;
        skipped = true;
    }
    skipped
}

/// Skip a brace-enclosed `{ ... }` comment at the cursor.
/// Returns true if a comment was skipped, false otherwise.
#[allow(dead_code)]
pub(crate) fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if *cursor >= bytes.len() || bytes[*cursor] != b'{' {
        return false;
    }
    *cursor += 1;

    let mut left = 1u32;
    let mut right = 0u32;
    while left != right {
        if *cursor >= bytes.len() {
            // C aborts here. Just stop to avoid panic in safe Rust.
            return true;
        }
        if bytes[*cursor] == b'{' {
            left += 1;
        }
        if bytes[*cursor] == b'}' {
            right += 1;
        }
        *cursor += 1;
    }
    true
}

/// Skip a newline (\n or \r\n) at the cursor. Returns true.
pub(crate) fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if *cursor < bytes.len() && bytes[*cursor] == b'\r' {
        *cursor += 1;
        if *cursor < bytes.len() && bytes[*cursor] == b'\n' {
            *cursor += 1;
        }
        return true;
    }
    if *cursor < bytes.len() && bytes[*cursor] == b'\n' {
        *cursor += 1;
    }
    true
}
