// Translation of c_src/src/lib.c to Rust.
//
// The original C uses C-string functions (strcmp, strncmp, strlen,
// strncpy, strncat, snprintf) on buffers that may not be NUL-terminated.
// In our translation the input and reference buffers are sized at
// MAX_BUFFER_SIZE (1024) and zero-initialized, so all NUL-terminated
// reads are well-defined: a NUL is guaranteed to be reachable somewhere
// in the buffer.

/// Length of a NUL-terminated byte slice (mimics libc strlen).
fn c_strlen(buf: &[u8]) -> usize {
    let mut i = 0usize;
    while i < buf.len() && buf[i] != 0 {
        i += 1;
    }
    i
}

/// Compare two NUL-terminated byte slices (mimics libc strcmp).
/// Returns negative/zero/positive like C's strcmp. We only need to
/// know whether the result is zero, but matching the sign exactly
/// keeps the function semantically identical to strcmp.
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

/// Compare up to n bytes of two NUL-terminated byte slices
/// (mimics libc strncmp).
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
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
    0
}

/// Mimics `strncpy(dest, src, n)`: copies up to `n` bytes from `src` to
/// `dest`. If `src`'s NUL terminator is reached within those `n` bytes,
/// the remainder of `dest` (up to position `n`) is zero-padded. If
/// `src` is longer than `n`, no NUL terminator is written. `dest` is
/// assumed to have at least `n` bytes available.
fn c_strncpy(dest: &mut [u8], src: &[u8], n: usize) {
    let mut i = 0usize;
    let mut hit_nul = false;
    while i < n {
        if !hit_nul {
            let c = if i < src.len() { src[i] } else { 0 };
            dest[i] = c;
            if c == 0 {
                hit_nul = true;
            }
        } else {
            dest[i] = 0;
        }
        i += 1;
    }
}

/// Mimics `strncat(dest, src, n)`: appends up to `n` bytes from `src`
/// to the NUL-terminated string in `dest`, then writes a terminating
/// NUL. The destination buffer must have enough space for the NUL.
fn c_strncat(dest: &mut [u8], src: &[u8], n: usize) {
    let dlen = c_strlen(dest);
    let mut i = 0usize;
    while i < n {
        let c = if i < src.len() { src[i] } else { 0 };
        if c == 0 {
            break;
        }
        if dlen + i >= dest.len() {
            break;
        }
        dest[dlen + i] = c;
        i += 1;
    }
    if dlen + i < dest.len() {
        dest[dlen + i] = 0;
    }
}

/// Public process_strings: matches the C signature behaviorally.
/// `input` corresponds to the writable input buffer; `reference` is
/// the read-only reference buffer; their respective `*_len` arguments
/// give the user-supplied lengths. The buffers are at least
/// MAX_BUFFER_SIZE bytes and zero-padded.
pub fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // The C code does `if (input == NULL) return -1;` — in our
    // translation we always have a non-null slice, so this branch
    // is unreachable. Keep the structure for clarity.
    // (Rust references can't be null.)

    match operation {
        0 => {
            // Validate token.
            // C: `if (reference == NULL) return -2;` — we model NULL
            // reference as ref_len being 0 *and* the buffer not used.
            // In the C harness, reference is always a valid pointer,
            // so this branch is also unreachable in practice.
            let _ = ref_len;
            validate_token(input, reference)
        }
        1 => {
            // Parse command.
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input, input_len, &commands)
        }
        2 => {
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            // Find delimiter position.
            // C: `char delim = (reference && ref_len > 0) ? reference[0] : ':';`
            let delim: u8 = if ref_len > 0 { reference[0] } else { b':' };
            find_delimiter(input, input_len, delim)
        }
        4 => {
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
        let cmd_len = cmd.len(); // commands are byte-string literals without NUL
        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd, cmd_len) == 0 {
                // VULNERABLE: read at index cmd_len. Since `buffer` is
                // MAX_BUFFER_SIZE bytes zero-padded, this index is in
                // bounds and well-defined.
                let next = if cmd_len < buffer.len() { buffer[cmd_len] } else { 0 };
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

        // C: `char variations[5][32] = {"_v1", "_v2", "_old", "_new", "_tmp"};`
        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];

        for (i, var) in variations.iter().enumerate() {
            // C: `char expected[64]; strncpy(expected, prefix, 63);`
            // expected is uninitialized in C; we zero-initialize.
            let mut expected = [0u8; 64];
            c_strncpy(&mut expected[..63], prefix, 63);
            expected[63] = 0;
            // C: `strncat(expected, variations[i], 63 - strlen(expected));`
            // strncat writes at most n+1 bytes (n from src plus NUL).
            // For our 64-byte buffer this is safe.
            let elen = c_strlen(&expected);
            let n = 63usize.saturating_sub(elen);
            // Pass a sub-slice that lets strncat write its NUL within
            // bounds of `expected`.
            c_strncat(&mut expected, var, n);

            if c_strcmp(s, &expected) == 0 {
                return (2 + i) as i32;
            }
        }
        0
    } else {
        // strncmp returns 0 when prefix_len == 0 even for empty
        // prefix (matches C semantics).
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
    let mut i = 0usize;
    while i < len {
        if i >= data.len() {
            break;
        }
        if data[i] == delim {
            return i as i32;
        }
        if data[i] == 0 {
            break;
        }
        i += 1;
    }
    if delim == b'|' && c_strcmp(data, b"NONE\0") == 0 {
        return -2;
    }
    if delim == b':' && c_strcmp(data, b"EMPTY\0") == 0 {
        return -3;
    }
    -1
}

/// snprintf-equivalent for the three wildcard patterns. Each call must
/// produce the exact same NUL-terminated bytes as
/// `snprintf(dest, 64, fmt, pattern)` where `pattern` is a
/// NUL-terminated C string. The format strings used in C are
/// "*%s*", "%s*", "*%s".
///
/// snprintf into a 64-byte buffer writes at most 63 characters of
/// output plus a NUL. If the formatted output would exceed that, it
/// is truncated.
fn snprintf_wildcard(dest: &mut [u8; 64], prefix: &[u8], pattern: &[u8], suffix: &[u8]) {
    // Build the formatted bytes. `pattern` is a NUL-terminated buffer
    // and we stop at its NUL just like %s would.
    let plen = c_strlen(pattern);
    let mut out_idx = 0usize;
    let cap = 63usize; // 63 bytes of content + 1 NUL

    // Write prefix
    for &b in prefix.iter() {
        if out_idx >= cap {
            break;
        }
        dest[out_idx] = b;
        out_idx += 1;
    }
    // Write pattern (up to its NUL)
    let mut i = 0usize;
    while i < plen && out_idx < cap {
        dest[out_idx] = pattern[i];
        out_idx += 1;
        i += 1;
    }
    // Write suffix
    for &b in suffix.iter() {
        if out_idx >= cap {
            break;
        }
        dest[out_idx] = b;
        out_idx += 1;
    }
    // NUL-terminate. snprintf always writes a terminating NUL when
    // the destination size is >= 1.
    dest[out_idx] = 0;
    // Zero out the rest for cleanliness (matches the typical
    // behavior of snprintf into a fresh buffer plus our intentional
    // zero-init).
    for j in (out_idx + 1)..dest.len() {
        dest[j] = 0;
    }
}

/// Match pattern with optional case sensitivity.
fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        // Build the three wildcard patterns the same way snprintf does.
        let mut wildcard_patterns: [[u8; 64]; 3] = [[0u8; 64]; 3];
        snprintf_wildcard(&mut wildcard_patterns[0], b"*", pattern, b"*");
        snprintf_wildcard(&mut wildcard_patterns[1], b"", pattern, b"*");
        snprintf_wildcard(&mut wildcard_patterns[2], b"*", pattern, b"");

        for i in 0..3 {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return (2 + i) as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        // C loop: `for (size_t i = 0; i <= text_len - pattern_len; i++)`
        // If pattern_len > text_len, the subtraction underflows in C
        // (size_t is unsigned), producing a huge upper bound and
        // out-of-bounds reads. We model that as "no further matches"
        // because the well-defined intent is to find pattern within
        // text, which is impossible if pattern is longer.
        if pattern_len <= text_len {
            // Inclusive upper bound: 0 ..= text_len - pattern_len.
            let last = text_len - pattern_len;
            let mut i = 0usize;
            loop {
                if c_strncmp(&text[i..], pattern, pattern_len) == 0 {
                    return (10 + i) as i32;
                }
                if i == last {
                    break;
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
                    c1 += 32;
                }
                if c2 >= b'A' && c2 <= b'Z' {
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
