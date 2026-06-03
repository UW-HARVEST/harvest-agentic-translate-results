//! Translation of c_src/src/lib.c into safe Rust.
//!
//! All C string semantics are reproduced via slice helpers: every byte buffer
//! is treated as a NUL-terminated C string (we walk to the first 0 byte). The
//! original C operates on stack-allocated 1024-byte buffers that are read into
//! up to `input_len` bytes; bytes beyond that are uninitialized in C but here
//! are zero-filled by `main.rs`. That deterministic behavior mirrors what most
//! C runtimes give in practice for this benchmark and yields stable output.
//!
//! Several functions in lib.c are tagged "VULNERABLE" — the unsafe `strcmp`
//! calls. Per the translation requirements we faithfully reproduce that
//! behavior (treating the buffer as a NUL-terminated C string) rather than
//! trying to be safer.

/// strcmp semantics over a pair of buffers, each interpreted as a C string
/// (terminated at the first NUL byte or at end of buffer).
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let alen = c_strlen(a);
    let blen = c_strlen(b);
    let mut i = 0usize;
    loop {
        let ca = if i < alen { a[i] } else { 0 };
        let cb = if i < blen { b[i] } else { 0 };
        if ca != cb {
            // C returns the difference of unsigned char promoted to int.
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// strncmp semantics: compare up to `n` bytes, stopping early if a NUL is hit.
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let alen = c_strlen(a);
    let blen = c_strlen(b);
    for i in 0..n {
        let ca = if i < alen { a[i] } else { 0 };
        let cb = if i < blen { b[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
    }
    0
}

/// strlen semantics: return the index of the first NUL byte, or the buffer
/// length if no NUL byte exists.
fn c_strlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// strncpy semantics: copy up to `n` bytes from `src` to `dst`. If `src` is
/// shorter than `n` (including NUL), pad the rest of `dst[..n]` with NULs.
/// `dst` is NOT guaranteed to be NUL-terminated if `src` is at least `n` long.
fn c_strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let src_eff_len = c_strlen(src);
    let copy_len = src_eff_len.min(n);
    if copy_len > 0 {
        dst[..copy_len].copy_from_slice(&src[..copy_len]);
    }
    // Zero-pad the remainder up to `n` bytes.
    for i in copy_len..n {
        if i >= dst.len() {
            break;
        }
        dst[i] = 0;
    }
}

/// strncat semantics: append up to `n` bytes from `src` to `dst` (which must
/// be NUL-terminated), then NUL-terminate the result.
fn c_strncat(dst: &mut [u8], src: &[u8], n: usize) {
    let dst_len = c_strlen(dst);
    let src_eff_len = c_strlen(src);
    let copy_len = src_eff_len.min(n);
    for i in 0..copy_len {
        if dst_len + i >= dst.len() {
            return;
        }
        dst[dst_len + i] = src[i];
    }
    if dst_len + copy_len < dst.len() {
        dst[dst_len + copy_len] = 0;
    }
}

/// Process strings - main dispatcher.
/// Returns operation result (match count, position, or error code).
pub fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // input is never null in our Rust translation (it's a non-null reference).
    // The original `if (input == NULL) return -1;` check is therefore vacuous
    // here — we don't have a way to pass null. The C `reference == NULL`
    // checks are similarly vacuous.

    match operation {
        0 => {
            // Validate token
            validate_token(input, reference)
        }
        1 => {
            // Parse command from list
            let commands: [&[u8]; 5] = [
                b"START\0",
                b"STOP\0",
                b"PAUSE\0",
                b"RESUME\0",
                b"RESET\0",
            ];
            parse_command(input, input_len, &commands)
        }
        2 => {
            // Compare prefix
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            // Find delimiter position
            let delim = if ref_len > 0 { reference[0] } else { b':' };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            // Match pattern
            let case_sens = (flags & 0x02) != 0;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

/// Validate token against expected value.
fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1;
    }
    if c_strcmp(token, b"VALID\0") == 0 || c_strcmp(token, b"OK\0") == 0 {
        return 1;
    }
    0
}

/// Parse command from a list of valid commands.
fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = c_strlen(cmd);

        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd, cmd_len) == 0 {
                // Check the byte at offset cmd_len. Mirrors the C code which
                // accesses `buffer[cmd_len]` regardless of whether `cmd_len`
                // is within `buf_size`.
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

        // Fallback: direct strcmp.
        if c_strcmp(buffer, cmd) == 0 {
            return i as i32;
        }
    }

    if c_strcmp(buffer, b"ADMIN\0") == 0 {
        return 99;
    }

    -1
}

/// Compare prefix with optional exact matching.
fn compare_prefix(s: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match {
        if c_strcmp(s, prefix) == 0 {
            return 1;
        }

        // Variations: 5 buffers of size 32 (faithful to the C declaration).
        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (i, variation) in variations.iter().enumerate() {
            // Reproduce: strncpy(expected, prefix, 63); expected[63] = '\0';
            //           strncat(expected, variations[i], 63 - strlen(expected));
            let mut expected = [0u8; 64];
            c_strncpy(&mut expected, prefix, 63);
            expected[63] = 0;

            // Build a NUL-terminated copy of the variation for strncat.
            let mut var_buf = [0u8; 32];
            let vl = variation.len().min(31);
            var_buf[..vl].copy_from_slice(&variation[..vl]);
            // (var_buf[vl..] already zero)

            let cur_len = c_strlen(&expected);
            let n = 63usize.saturating_sub(cur_len);
            c_strncat(&mut expected, &var_buf, n);

            if c_strcmp(s, &expected) == 0 {
                return 2 + i as i32;
            }
        }

        return 0;
    } else {
        if c_strncmp(s, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

/// Find delimiter position in string.
fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    for i in 0..len {
        if i >= data.len() {
            break;
        }
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

/// Helper to build a wildcard pattern via snprintf-like semantics into a 64-byte
/// buffer (matching `char wildcard_patterns[3][64]` in C).
///
/// `kind` selects the format: 0 = "*%s*", 1 = "%s*", 2 = "*%s".
fn build_wildcard(kind: u8, pattern: &[u8]) -> [u8; 64] {
    let mut buf = [0u8; 64];
    let pattern_eff = c_strlen(pattern);
    let pattern = &pattern[..pattern_eff];
    let max_chars: usize = 63; // snprintf size 64 -> at most 63 chars + NUL
    let mut pos: usize = 0;

    let write = |c: u8, pos: &mut usize, buf: &mut [u8; 64]| {
        if *pos < max_chars {
            buf[*pos] = c;
            *pos += 1;
        }
    };

    match kind {
        0 => {
            write(b'*', &mut pos, &mut buf);
            for &c in pattern {
                write(c, &mut pos, &mut buf);
            }
            write(b'*', &mut pos, &mut buf);
        }
        1 => {
            for &c in pattern {
                write(c, &mut pos, &mut buf);
            }
            write(b'*', &mut pos, &mut buf);
        }
        2 => {
            write(b'*', &mut pos, &mut buf);
            for &c in pattern {
                write(c, &mut pos, &mut buf);
            }
        }
        _ => {}
    }
    if pos < buf.len() {
        buf[pos] = 0;
    } else {
        buf[63] = 0;
    }
    buf
}

/// Match pattern with optional case sensitivity.
fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        let wildcard0 = build_wildcard(0, pattern);
        let wildcard1 = build_wildcard(1, pattern);
        let wildcard2 = build_wildcard(2, pattern);
        let wildcards = [&wildcard0[..], &wildcard1[..], &wildcard2[..]];

        for (i, wc) in wildcards.iter().enumerate() {
            if c_strcmp(text, wc) == 0 {
                return 2 + i as i32;
            }
        }

        // Substring search using strncmp.
        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        // The C code computes `text_len - pattern_len` in size_t; if
        // pattern_len > text_len this underflows to a huge value and the
        // resulting access is undefined behavior. We guard against entering
        // the loop in that case to avoid panicking on out-of-bounds access;
        // for any input where text_len >= pattern_len our output matches the C.
        if text_len >= pattern_len {
            let last = text_len - pattern_len;
            let mut i = 0usize;
            while i <= last {
                if c_strncmp(&text[i..], pattern, pattern_len) == 0 {
                    return 10 + i as i32;
                }
                i += 1;
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
