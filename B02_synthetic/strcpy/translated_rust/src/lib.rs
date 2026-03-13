/// C-compatible string helpers operating on byte slices.
/// These replicate C's strcmp/strncmp/strlen semantics on raw byte buffers.

fn c_strlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb {
            return (ca as i32) - (cb as i32);
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    for i in 0..n {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb {
            return (ca as i32) - (cb as i32);
        }
        if ca == 0 {
            return 0;
        }
    }
    0
}

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1;
    }
    if c_strcmp(token, b"VALID\0") == 0 || c_strcmp(token, b"OK\0") == 0 {
        return 1;
    }
    0
}

fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = c_strlen(cmd);
        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd, cmd_len) == 0 {
                let b = if cmd_len < buffer.len() { buffer[cmd_len] } else { 0 };
                if b == 0 || b == b' ' {
                    return i as i32;
                }
            }
        }
        if c_strcmp(buffer, cmd) == 0 {
            return i as i32;
        }
    }
    if c_strcmp(buffer, b"ADMIN\0") == 0 {
        return 99;
    }
    -1
}

fn compare_prefix(str_buf: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match != 0 {
        if c_strcmp(str_buf, prefix) == 0 {
            return 1;
        }
        let variations: [&[u8]; 5] = [b"_v1\0", b"_v2\0", b"_old\0", b"_new\0", b"_tmp\0"];
        for (i, var) in variations.iter().enumerate() {
            let mut expected = [0u8; 64];
            // strncpy(expected, prefix, 63); expected[63]='\0';
            let plen = c_strlen(prefix);
            let copy_len = plen.min(63);
            expected[..copy_len].copy_from_slice(&prefix[..copy_len]);
            expected[63] = 0;
            // strncat(expected, variations[i], 63 - strlen(expected))
            let elen = c_strlen(&expected);
            let vlen = c_strlen(var);
            let space = 63usize.saturating_sub(elen);
            let cat_len = vlen.min(space);
            for j in 0..cat_len {
                expected[elen + j] = var[j];
            }
            expected[elen + cat_len] = 0;

            if c_strcmp(str_buf, &expected) == 0 {
                return 2 + i as i32;
            }
        }
        return 0;
    } else {
        if c_strncmp(str_buf, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }
    for i in 0..len {
        if data[i] == delim {
            return i as i32;
        }
        if data[i] == 0 {
            break;
        }
    }
    if delim == b'|' && c_strcmp(data, b"NONE\0") == 0 {
        return -2;
    }
    if delim == b':' && c_strcmp(data, b"EMPTY\0") == 0 {
        return -3;
    }
    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }
        // Build wildcard patterns: "*pat*", "pat*", "*pat"
        let pat_len = c_strlen(pattern);
        let mut wp = [[0u8; 64]; 3];
        // snprintf(wp[0], 64, "*%s*", pattern)
        {
            let mut pos = 0usize;
            wp[0][pos] = b'*'; pos += 1;
            let n = pat_len.min(61);
            wp[0][pos..pos+n].copy_from_slice(&pattern[..n]); pos += n;
            if pos < 63 { wp[0][pos] = b'*'; pos += 1; }
            wp[0][pos] = 0;
        }
        // snprintf(wp[1], 64, "%s*", pattern)
        {
            let n = pat_len.min(62);
            wp[1][..n].copy_from_slice(&pattern[..n]);
            wp[1][n] = b'*';
            wp[1][n+1] = 0;
        }
        // snprintf(wp[2], 64, "*%s", pattern)
        {
            wp[2][0] = b'*';
            let n = pat_len.min(62);
            wp[2][1..1+n].copy_from_slice(&pattern[..n]);
            wp[2][1+n] = 0;
        }

        for i in 0..3 {
            if c_strcmp(text, &wp[i]) == 0 {
                return 2 + i as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        // C bug: text_len - pattern_len is unsigned; if pattern_len > text_len
        // it wraps to a huge number. Reproduce with wrapping_sub.
        let limit = text_len.wrapping_sub(pattern_len);
        let mut i = 0usize;
        while i <= limit {
            if c_strncmp(&text[i..], pattern, pattern_len) == 0 {
                return 10 + i as i32;
            }
            i = i.wrapping_add(1);
            // If we wrapped past limit (which is huge), we'd loop forever.
            // In C this would read garbage and eventually segfault or find \0.
            // With our bounded slices, c_strncmp returns 0-byte vs pattern byte
            // which won't match, so we just need to stop at buffer end.
            if i > text.len() {
                break;
            }
        }
    } else {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }
        let pattern_len = c_strlen(pattern);
        let text_len = c_strlen(text);

        if text_len != pattern_len {
            if c_strncmp(text, pattern, pattern_len) == 0 {
                return 5;
            }
        }

        if text_len == pattern_len {
            let mut matched = true;
            for i in 0..pattern_len {
                let mut c1 = text[i];
                let mut c2 = pattern[i];
                if c1 >= b'A' && c1 <= b'Z' { c1 += 32; }
                if c2 >= b'A' && c2 <= b'Z' { c2 += 32; }
                if c1 != c2 {
                    matched = false;
                    break;
                }
            }
            if matched {
                return 6;
            }
        }
    }
    0
}

pub fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // input is never NULL in our Rust version (always a valid slice)
    // but we keep the structure

    match operation {
        0 => {
            validate_token(input, reference)
        }
        1 => {
            let commands: [&[u8]; 5] = [
                b"START\0", b"STOP\0", b"PAUSE\0", b"RESUME\0", b"RESET\0",
            ];
            parse_command(input, input_len, &commands)
        }
        2 => {
            let exact = (flags & 0x01) as i32;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = if ref_len > 0 { reference[0] } else { b':' };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            let case_sens = (flags & 0x02) as i32;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}
