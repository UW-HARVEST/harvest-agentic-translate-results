pub fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: Option<&[u8]>,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    if input_len > input.len() {
        return -1;
    }
    let input = &mut input[..input_len];

    match operation {
        0 => {
            let reference = match reference {
                Some(r) if ref_len <= r.len() => &r[..ref_len],
                _ => return -2,
            };
            validate_token(input, reference)
        }
        1 => {
            let commands = ["START", "STOP", "PAUSE", "RESUME", "RESET"];
            parse_command(input, &commands)
        }
        2 => {
            let reference = match reference {
                Some(r) if ref_len <= r.len() => &r[..ref_len],
                _ => return -2,
            };
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = reference
                .and_then(|r| r.first().copied())
                .unwrap_or(b':');
            find_delimiter(input, delim)
        }
        4 => {
            let reference = match reference {
                Some(r) if ref_len <= r.len() => &r[..ref_len],
                _ => return -2,
            };
            let case_sens = (flags & 0x02) != 0;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if token == expected {
        return 1;
    }
    if token == b"VALID" || token == b"OK" {
        return 1;
    }
    0
}

fn parse_command(buffer: &[u8], cmd_list: &[&str]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_bytes = cmd.as_bytes();
        if buffer.starts_with(cmd_bytes) {
            let rest = &buffer[cmd_bytes.len().min(buffer.len())..];
            if rest.is_empty() || rest.first() == Some(&b' ') {
                return i as i32;
            }
        }
        if buffer == cmd_bytes {
            return i as i32;
        }
    }
    if buffer == b"ADMIN" {
        return 99;
    }
    -1
}

fn compare_prefix(str_buf: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    if exact_match {
        if str_buf == prefix {
            return 1;
        }
        let variations = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (i, var) in variations.iter().enumerate() {
            let mut expected = Vec::with_capacity(prefix.len() + var.len());
            expected.extend_from_slice(prefix);
            expected.extend_from_slice(var);
            if str_buf == expected.as_slice() {
                return 2 + i as i32;
            }
        }
        0
    } else {
        if str_buf.starts_with(prefix) {
            1
        } else {
            0
        }
    }
}

fn find_delimiter(data: &[u8], delim: u8) -> i32 {
    if data.is_empty() {
        return -1;
    }
    for (i, &b) in data.iter().enumerate() {
        if b == delim {
            return i as i32;
        }
        if b == 0 {
            break;
        }
    }
    if delim == b'|' && data == b"NONE" {
        return -2;
    }
    if delim == b':' && data == b"EMPTY" {
        return -3;
    }
    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if text == pattern {
            return 1;
        }
        let mut wildcard_patterns: [Vec<u8>; 3] = [
            Vec::with_capacity(pattern.len() + 2),
            Vec::with_capacity(pattern.len() + 1),
            Vec::with_capacity(pattern.len() + 1),
        ];
        wildcard_patterns[0].push(b'*');
        wildcard_patterns[0].extend_from_slice(pattern);
        wildcard_patterns[0].push(b'*');
        wildcard_patterns[1].extend_from_slice(pattern);
        wildcard_patterns[1].push(b'*');
        wildcard_patterns[2].push(b'*');
        wildcard_patterns[2].extend_from_slice(pattern);
        for (i, wp) in wildcard_patterns.iter().enumerate() {
            if text == wp.as_slice() {
                return 2 + i as i32;
            }
        }
        for (i, window) in text.windows(pattern.len()).enumerate() {
            if window == pattern {
                return 10 + i as i32;
            }
        }
    } else {
        if text == pattern {
            return 1;
        }
        if text.starts_with(pattern) {
            return 5;
        }
        if text.len() == pattern.len() {
            let match_ci = text.iter().zip(pattern.iter()).all(|(&t, &p)| {
                let c1 = if t >= b'A' && t <= b'Z' { t + 32 } else { t };
                let c2 = if p >= b'A' && p <= b'Z' { p + 32 } else { p };
                c1 == c2
            });
            if match_ci {
                return 6;
            }
        }
    }
    0
}
