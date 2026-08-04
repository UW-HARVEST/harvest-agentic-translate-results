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

    let mut left_brace_count: u32 = 1;
    let mut right_brace_count: u32 = 0;

    while right_brace_count != left_brace_count {
        if *cursor >= bytes.len() || bytes[*cursor] == 0 {
            // Mirrors abort() in C
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

pub fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();

    if *cursor < bytes.len() && bytes[*cursor] == b'\r' {
        assert_eq!(bytes[*cursor], b'\r');
        *cursor += 1;
        assert_eq!(bytes[*cursor], b'\n');
        *cursor += 1;
        return true;
    }

    assert_eq!(bytes[*cursor], b'\n');
    *cursor += 1;
    true
}
