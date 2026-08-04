#[allow(dead_code)]
fn pgn_cursor_skip_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let start = *cursor;
    while matches!(bytes.get(*cursor), Some(b) if (*b as char).is_ascii_whitespace()) {
        *cursor += 1;
    }
    *cursor != start
}

#[allow(dead_code)]
fn pgn_cursor_revisit_whitespace(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    let start = *cursor;
    while *cursor > 0
        && matches!(bytes.get(*cursor - 1), Some(b) if (*b as char).is_ascii_whitespace())
    {
        *cursor -= 1;
    }
    *cursor != start
}

#[allow(dead_code)]
fn pgn_cursor_skip_comment(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if !matches!(bytes.get(*cursor), Some(b'{')) {
        return false;
    }

    *cursor += 1;
    let mut left = 1usize;
    let mut right = 0usize;
    while right != left {
        match bytes.get(*cursor) {
            Some(b'{') => left += 1,
            Some(b'}') => right += 1,
            Some(_) => {}
            None => return false,
        }
        *cursor += 1;
    }

    true
}

#[allow(dead_code)]
fn pgn_cursor_skip_newline(s: &str, cursor: &mut usize) -> bool {
    let bytes = s.as_bytes();
    if matches!(bytes.get(*cursor), Some(b'\r')) {
        if matches!(bytes.get(*cursor + 1), Some(b'\n')) {
            *cursor += 2;
            return true;
        }
        return false;
    }

    if matches!(bytes.get(*cursor), Some(b'\n')) {
        *cursor += 1;
        return true;
    }

    false
}
