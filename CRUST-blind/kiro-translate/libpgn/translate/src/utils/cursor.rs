pub fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor < bytes.len() && (bytes[*cursor] as char).is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

pub fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let mut skipped = false;
    while *cursor > 0 && (bytes[*cursor - 1] as char).is_ascii_whitespace() {
        *cursor -= 1;
        skipped = true;
    }
    skipped
}

pub fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if *cursor >= bytes.len() || bytes[*cursor] != b'{' {
        return false;
    }
    *cursor += 1;
    let mut left = 1u32;
    let mut right = 0u32;
    while right != left {
        assert!(*cursor < bytes.len(), "unexpected end of string in comment");
        if bytes[*cursor] == b'{' { left += 1; }
        if bytes[*cursor] == b'}' { right += 1; }
        *cursor += 1;
    }
    true
}

pub fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if *cursor < bytes.len() && bytes[*cursor] == b'\r' {
        *cursor += 1;
        assert!(*cursor < bytes.len() && bytes[*cursor] == b'\n');
        *cursor += 1;
        return true;
    }
    assert!(*cursor < bytes.len() && bytes[*cursor] == b'\n');
    *cursor += 1;
    true
}
