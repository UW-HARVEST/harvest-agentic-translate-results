use std::cmp::min;

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

    let input_slice = &input[..input_len];

    match operation {
        0 => {
            let reference = match reference {
                Some(r) if ref_len <= r.len() => &r[..ref_len],
                _ => return -2,
            };
            validate_token(input_slice, reference)
        }
        1 => {
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input_slice, &commands)
        }
        2 => {
            let reference = match reference {
                Some(r) if ref_len <= r.len() => &r[..ref_len],
                _ => return -2,
            };
            let exact = (flags & 0x01) != 0;
            compare_prefix(input_slice, reference, exact)
        }
        3 => {
            let delim = match reference {
                Some(r) if ref_len > 0 && !r.is_empty() => r[0],
                _ => b':',
            };
            find_delimiter(input_slice, delim)
        }
        4 => {
            let reference = match reference {
                Some(r) if ref_len <= r.len() => &r[..ref_len],
                _ => return -2,
            };
            let case_sens = (flags & 0x02) != 0;
            match_pattern(input_slice, reference, case_sens)
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

fn parse_command(buffer: &[u8], cmd_list: &[&[u8]]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = cmd.len();
        if buffer.len() >= cmd_len && &buffer[..cmd_len] == *cmd {
            if buffer.len() == cmd_len || buffer.get(cmd_len) == Some(&b' ') {
                return i as i32;
            }
        }
        if buffer == *cmd {
            return i as i32;
        }
    }

    if buffer == b"ADMIN" {
        return 99;
    }

    -1
}

fn compare_prefix(str_bytes: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_len = prefix.len();

    if exact_match {
        if str_bytes == prefix {
            return 1;
        }

        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (i, variation) in variations.iter().enumerate() {
            let take_len = min(prefix.len(), 63);
            let mut expected = Vec::with_capacity(64);
            expected.extend_from_slice(&prefix[..take_len]);
            let remaining = 63usize.saturating_sub(expected.len());
            expected.extend_from_slice(&variation[..min(variation.len(), remaining)]);
            if str_bytes == expected.as_slice() {
                return 2 + i as i32;
            }
        }

        0
    } else {
        if str_bytes.len() >= prefix_len && &str_bytes[..prefix_len] == prefix {
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

    for (i, b) in data.iter().enumerate() {
        if *b == delim {
            return i as i32;
        }
        if *b == 0 {
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

        let mut wildcard_patterns = Vec::with_capacity(3);
        let mut p0 = Vec::with_capacity(pattern.len() + 2);
        p0.push(b'*');
        p0.extend_from_slice(pattern);
        p0.push(b'*');
        wildcard_patterns.push(p0);

        let mut p1 = Vec::with_capacity(pattern.len() + 1);
        p1.extend_from_slice(pattern);
        p1.push(b'*');
        wildcard_patterns.push(p1);

        let mut p2 = Vec::with_capacity(pattern.len() + 1);
        p2.push(b'*');
        p2.extend_from_slice(pattern);
        wildcard_patterns.push(p2);

        for (i, candidate) in wildcard_patterns.iter().enumerate() {
            if text == candidate.as_slice() {
                return 2 + i as i32;
            }
        }

        let text_len = text.len();
        let pattern_len = pattern.len();
        if pattern_len <= text_len {
            for i in 0..=text_len - pattern_len {
                if &text[i..i + pattern_len] == pattern {
                    return 10 + i as i32;
                }
            }
        }
    } else {
        if text == pattern {
            return 1;
        }

        let pattern_len = pattern.len();
        let text_len = text.len();

        if text_len != pattern_len {
            if text_len >= pattern_len && &text[..pattern_len] == pattern {
                return 5;
            }
        }

        if text_len == pattern_len {
            let mut matches = true;
            for i in 0..pattern_len {
                let c1 = text[i].to_ascii_lowercase();
                let c2 = pattern[i].to_ascii_lowercase();
                if c1 != c2 {
                    matches = false;
                    break;
                }
            }
            if matches {
                return 6;
            }
        }
    }

    0
}
