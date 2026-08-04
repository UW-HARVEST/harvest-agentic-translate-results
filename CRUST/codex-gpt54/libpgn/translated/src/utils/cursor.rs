fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let mut skipped = false;
    let bytes = s.as_bytes();
    while *cursor < bytes.len() && bytes[*cursor].is_ascii_whitespace() {
        *cursor += 1;
        skipped = true;
    }
    skipped
}

fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let mut skipped = false;
    let bytes = s.as_bytes();
    while *cursor > 0 && bytes[*cursor - 1].is_ascii_whitespace() {
        *cursor -= 1;
        skipped = true;
    }
    skipped
}

fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if bytes.get(*cursor) != Some(&b'{') {
        return false;
    }

    *cursor += 1;
    let mut left_brace_count = 1usize;
    let mut right_brace_count = 0usize;

    while right_brace_count != left_brace_count {
        assert!(*cursor < bytes.len());
        left_brace_count += usize::from(bytes[*cursor] == b'{');
        right_brace_count += usize::from(bytes[*cursor] == b'}');
        *cursor += 1;
    }

    true
}

fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if bytes.get(*cursor) == Some(&b'\r') {
        assert_eq!(bytes.get(*cursor), Some(&b'\r'));
        *cursor += 1;
        assert_eq!(bytes.get(*cursor), Some(&b'\n'));
        *cursor += 1;
        return true;
    }

    assert_eq!(bytes.get(*cursor), Some(&b'\n'));
    *cursor += 1;
    true
}
