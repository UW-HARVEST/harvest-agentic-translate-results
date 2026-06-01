// Translation of c_src/src/lib.c

/// Returns the length of a C-style null-terminated byte string within `buf`.
/// If no null byte is found, returns `buf.len()`.
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// Mimics C's `strcmp` returning negative/0/positive.
/// Iterates byte-by-byte treating bytes as unsigned char until a null is found
/// in both, or a difference is found.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
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

/// Mimics C's `strncmp`.
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

/// Mimics C's `strncpy(dst, src, n)`: copies up to n bytes from src into dst,
/// stopping at first null in src and zero-padding the remainder of n bytes.
fn c_strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let mut found_null = false;
    for i in 0..n {
        if i >= dst.len() {
            break;
        }
        if found_null {
            dst[i] = 0;
        } else if i < src.len() && src[i] != 0 {
            dst[i] = src[i];
        } else {
            dst[i] = 0;
            found_null = true;
        }
    }
}

/// Mimics C's `strncat(dst, src, n)`: append up to n bytes from src to dst (a
/// C-string), terminating with a null. Always writes a final null byte.
fn c_strncat(dst: &mut [u8], src: &[u8], n: usize) {
    let dlen = c_strlen(dst);
    let mut i = 0usize;
    while i < n {
        let pos = dlen + i;
        if pos >= dst.len() {
            return; // out of room, no null appended (UB territory)
        }
        let c = if i < src.len() { src[i] } else { 0 };
        dst[pos] = c;
        if c == 0 {
            return;
        }
        i += 1;
    }
    let pos = dlen + n;
    if pos < dst.len() {
        dst[pos] = 0;
    }
}

/// Mimics `snprintf(buf, n, fmt, ...)` for the specific format strings used
/// in this file. Returns nothing (we don't need the return value here).
/// `pieces` is a list of byte slices to concatenate. Writes up to n bytes
/// including the trailing null.
fn c_snprintf_concat(buf: &mut [u8], n: usize, pieces: &[&[u8]]) {
    if n == 0 || buf.is_empty() {
        return;
    }
    let max_write = n.min(buf.len());
    let mut idx = 0usize;
    'outer: for piece in pieces {
        for &b in piece.iter() {
            if b == 0 {
                break;
            }
            if idx + 1 >= max_write {
                break 'outer;
            }
            buf[idx] = b;
            idx += 1;
        }
    }
    if idx < max_write {
        buf[idx] = 0;
    } else if max_write > 0 {
        buf[max_write - 1] = 0;
    }
}

/// Main entrance function – performs various string comparison operations.
pub fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // The C code checks `if (input == NULL)`. Slices are never null in Rust;
    // we mirror semantics by treating an empty slice as non-null. Caller
    // always provides the full 1024-byte buffer.
    let _ = input.len(); // touch input to avoid warnings

    match operation {
        0 => {
            // Validate token
            if reference.is_empty() {
                return -2;
            }
            validate_token(input, reference)
        }
        1 => {
            // Parse command from list
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input, input_len, &commands)
        }
        2 => {
            if reference.is_empty() {
                return -2;
            }
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = if !reference.is_empty() && ref_len > 0 {
                reference[0]
            } else {
                b':'
            };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            if reference.is_empty() {
                return -2;
            }
            let case_sens = (flags & 0x02) != 0;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1;
    }
    if c_strcmp(token, b"VALID") == 0 || c_strcmp(token, b"OK") == 0 {
        return 1;
    }
    0
}

fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = c_strlen(cmd);

        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd, cmd_len) == 0 {
                // VULNERABLE: buffer[cmd_len] may be past valid input but is
                // within the 1024-byte allocation in C; we mirror that here.
                let next = if cmd_len < buffer.len() {
                    buffer[cmd_len]
                } else {
                    0
                };
                if next == 0 || next == b' ' {
                    return i as i32;
                }
            }
        }

        // Fallback: direct strcmp – VULNERABLE
        if c_strcmp(buffer, cmd) == 0 {
            return i as i32;
        }
    }

    if c_strcmp(buffer, b"ADMIN") == 0 {
        return 99;
    }

    -1
}

fn compare_prefix(s: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match {
        if c_strcmp(s, prefix) == 0 {
            return 1;
        }

        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (i, var) in variations.iter().enumerate() {
            // Construct expected via strncpy/strncat as in C.
            let mut expected = [0u8; 64];
            c_strncpy(&mut expected, prefix, 63);
            expected[63] = 0;
            let elen = c_strlen(&expected);
            let remaining = 63usize.saturating_sub(elen);
            c_strncat(&mut expected, var, remaining);

            if c_strcmp(s, &expected) == 0 {
                return (2 + i) as i32;
            }
        }
        return 0;
    } else {
        if c_strncmp(s, prefix, prefix_len) == 0 {
            return 1;
        }
        return 0;
    }
}

fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    for i in 0..len {
        let b = if i < data.len() { data[i] } else { 0 };
        if b == delim {
            return i as i32;
        }
        if b == 0 {
            break;
        }
    }

    if delim == b'|' && c_strcmp(data, b"NONE") == 0 {
        return -2;
    }

    if delim == b':' && c_strcmp(data, b"EMPTY") == 0 {
        return -3;
    }

    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        let mut wildcard_patterns: [[u8; 64]; 3] = [[0u8; 64]; 3];
        c_snprintf_concat(&mut wildcard_patterns[0], 64, &[b"*", pattern, b"*"]);
        c_snprintf_concat(&mut wildcard_patterns[1], 64, &[pattern, b"*"]);
        c_snprintf_concat(&mut wildcard_patterns[2], 64, &[b"*", pattern]);

        for i in 0..3 {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return (2 + i) as i32;
            }
        }

        // Substring search using strncmp; mirrors C's `for (size_t i = 0;
        // i <= text_len - pattern_len; i++)`. If `pattern_len > text_len`
        // this would underflow in C and likely segfault. We avoid that here
        // because, in practice, well-formed inputs never trigger it.
        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        if pattern_len <= text_len {
            for i in 0..=(text_len - pattern_len) {
                let slice = if i < text.len() { &text[i..] } else { &[][..] };
                if c_strncmp(slice, pattern, pattern_len) == 0 {
                    return (10 + i) as i32;
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
                let mut c1 = if i < text.len() { text[i] } else { 0 };
                let mut c2 = if i < pattern.len() { pattern[i] } else { 0 };
                if c1 >= b'A' && c1 <= b'Z' {
                    c1 = c1.wrapping_add(32);
                }
                if c2 >= b'A' && c2 <= b'Z' {
                    c2 = c2.wrapping_add(32);
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
