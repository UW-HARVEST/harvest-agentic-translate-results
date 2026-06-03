/*
 * Translated from C to Rust.
 * Original: Copyright 2025 MIT Lincoln Laboratory
 */

// Helpers that mimic C string functions on byte buffers. We treat the
// buffers as null-terminated when they contain a 0 byte; otherwise the
// length is bounded by the buffer length.

/// Equivalent of C strlen but bounded by `buf.len()`.
fn c_strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&c| c == 0) {
        Some(p) => p,
        None => buf.len(),
    }
}

/// Equivalent of C strcmp on bounded byte buffers. Returns 0 when the
/// null-terminated contents match, non-zero otherwise.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let la = c_strlen(a);
    let lb = c_strlen(b);
    let n = la.min(lb);
    for i in 0..n {
        let ca = a[i];
        let cb = b[i];
        if ca != cb {
            return (ca as i32) - (cb as i32);
        }
    }
    (la as i32) - (lb as i32)
}

/// Equivalent of C strncmp on bounded byte buffers.
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

/// Equivalent of C strncpy: copies up to `n` bytes from `src` into `dest`,
/// padding with zero bytes if `src` (until its null terminator) is shorter
/// than `n`. Does NOT necessarily null-terminate `dest` if `src` is at
/// least `n` bytes (just like C).
fn c_strncpy(dest: &mut [u8], src: &[u8], n: usize) {
    let limit = n.min(dest.len());
    let mut i = 0;
    let mut hit_null = false;
    while i < limit {
        if hit_null {
            dest[i] = 0;
        } else {
            let c = if i < src.len() { src[i] } else { 0 };
            dest[i] = c;
            if c == 0 {
                hit_null = true;
            }
        }
        i += 1;
    }
}

/// Equivalent of C strncat: appends at most `n` bytes from `src` to the
/// null-terminated string in `dest`, then writes a null terminator.
fn c_strncat(dest: &mut [u8], src: &[u8], n: usize) {
    let dlen = c_strlen(dest);
    let mut i = 0;
    while i < n && (dlen + i) < dest.len() {
        let c = if i < src.len() { src[i] } else { 0 };
        if c == 0 {
            break;
        }
        dest[dlen + i] = c;
        i += 1;
    }
    if dlen + i < dest.len() {
        dest[dlen + i] = 0;
    }
}

/// Format using snprintf-style "%s" substitution. Writes up to `cap-1`
/// bytes into `dest` and a null terminator. The pieces are concatenated
/// in order.
fn snprintf_into(dest: &mut [u8], cap: usize, pieces: &[&[u8]]) {
    let limit = cap.min(dest.len());
    let mut idx = 0;
    'outer: for piece in pieces {
        for &c in *piece {
            if c == 0 {
                break;
            }
            if idx + 1 >= limit {
                break 'outer;
            }
            dest[idx] = c;
            idx += 1;
        }
    }
    if idx < limit {
        dest[idx] = 0;
    } else if limit > 0 {
        dest[limit - 1] = 0;
    }
}

/// Validate token against expected value
fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1;
    }
    if c_strcmp(token, b"VALID\0") == 0 || c_strcmp(token, b"OK\0") == 0 {
        return 1;
    }
    0
}

/// Parse command from a list of valid commands
fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]]) -> i32 {
    let view = if buf_size <= buffer.len() {
        &buffer[..buf_size]
    } else {
        buffer
    };

    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = c_strlen(cmd);

        if buf_size >= cmd_len {
            if c_strncmp(view, cmd, cmd_len) == 0 {
                // Check if exact match
                let next = if cmd_len < view.len() { view[cmd_len] } else { 0 };
                if next == 0 || next == b' ' {
                    return i as i32;
                }
            }
        }

        if c_strcmp(view, cmd) == 0 {
            return i as i32;
        }
    }

    if c_strcmp(view, b"ADMIN\0") == 0 {
        return 99;
    }

    -1
}

/// Compare prefix with optional exact matching
fn compare_prefix(s: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match {
        if c_strcmp(s, prefix) == 0 {
            return 1;
        }

        let variations: [&[u8]; 5] = [b"_v1\0", b"_v2\0", b"_old\0", b"_new\0", b"_tmp\0"];
        for (i, var) in variations.iter().enumerate() {
            let mut expected = [0u8; 64];
            c_strncpy(&mut expected, prefix, 63);
            expected[63] = 0;
            let cur_len = c_strlen(&expected);
            // strncat(expected, var, 63 - strlen(expected))
            let allowed = 63usize.saturating_sub(cur_len);
            c_strncat(&mut expected, var, allowed);

            if c_strcmp(s, &expected) == 0 {
                return 2 + i as i32;
            }
        }

        0
    } else {
        if c_strncmp(s, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

/// Find delimiter position in string
fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    let limit = len.min(data.len());
    for i in 0..limit {
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

/// Match pattern with optional case sensitivity
fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        // Build wildcard patterns: "*pattern*", "pattern*", "*pattern"
        let mut wildcard_patterns: [[u8; 64]; 3] = [[0u8; 64]; 3];
        snprintf_into(&mut wildcard_patterns[0], 64, &[b"*", pattern, b"*"]);
        snprintf_into(&mut wildcard_patterns[1], 64, &[pattern, b"*"]);
        snprintf_into(&mut wildcard_patterns[2], 64, &[b"*", pattern]);

        for i in 0..3 {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return 2 + i as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        if pattern_len <= text_len {
            let upper = text_len - pattern_len;
            for i in 0..=upper {
                if c_strncmp(&text[i..], pattern, pattern_len) == 0 {
                    return 10 + i as i32;
                }
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
                if (b'A'..=b'Z').contains(&c1) {
                    c1 += 32;
                }
                if (b'A'..=b'Z').contains(&c2) {
                    c2 += 32;
                }
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

/// Main entrance function - performs various string comparison operations.
///
/// `input` and `reference` are byte buffers (treated like C buffers that may
/// or may not be null-terminated). `input_len` and `ref_len` describe the
/// number of meaningful bytes the caller has placed into the buffers.
pub fn process_strings(
    input: &[u8],
    input_len: usize,
    reference: Option<&[u8]>,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // input being None corresponds to NULL in C; the Rust API requires a slice
    // so callers should not pass an empty buffer to indicate NULL. This mirrors
    // the Option treatment we apply to `reference`.
    if input.is_empty() && input_len > 0 {
        return -1;
    }

    match operation {
        0 => {
            let reference = match reference {
                Some(r) => r,
                None => return -2,
            };
            validate_token(input, reference)
        }
        1 => {
            let commands: [&[u8]; 5] = [b"START\0", b"STOP\0", b"PAUSE\0", b"RESUME\0", b"RESET\0"];
            parse_command(input, input_len, &commands)
        }
        2 => {
            let reference = match reference {
                Some(r) => r,
                None => return -2,
            };
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = match reference {
                Some(r) if ref_len > 0 && !r.is_empty() => r[0],
                _ => b':',
            };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            let reference = match reference {
                Some(r) => r,
                None => return -2,
            };
            let case_sens = (flags & 0x02) != 0;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}
