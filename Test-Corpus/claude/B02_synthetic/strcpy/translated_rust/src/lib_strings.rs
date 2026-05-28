// Translation of c_src/src/lib.c into safe Rust.
//
// Notes on faithfulness:
// - The C buffers `input_buffer` and `ref_buffer` are uninitialized stack
//   arrays of MAX_BUFFER_SIZE bytes; only the first `input_len` / `ref_len`
//   are filled. The C `strcmp`/`strncmp` calls read past those lengths and
//   rely on terminating null bytes that may or may not exist (undefined
//   behavior). In Rust we treat the supplied byte arrays the same way as the
//   C code does: we treat them as C-style strings terminated by the first
//   NUL byte. The caller passes zero-initialized buffers so reads past
//   `len` will reliably stop at the first zero byte.

/// Compute the C strlen of the buffer (number of bytes before first NUL).
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// Compare two C strings (NUL-terminated) like C `strcmp`.
/// Returns negative / zero / positive like C.
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

/// C strncmp: compare up to n bytes, stopping early at a NUL.
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

/// Mimics C `strncpy(dest, src, n)`: copies up to n bytes, padding with NUL
/// if src is shorter than n. Does NOT null-terminate dest if src has >= n
/// bytes before its NUL.
fn c_strncpy(dest: &mut [u8], src: &[u8], n: usize) {
    let mut copied_nul = false;
    for i in 0..n {
        if i >= dest.len() {
            break;
        }
        if copied_nul {
            dest[i] = 0;
        } else {
            let c = if i < src.len() { src[i] } else { 0 };
            dest[i] = c;
            if c == 0 {
                copied_nul = true;
            }
        }
    }
}

/// Mimics C `strncat(dest, src, n)`: appends up to n bytes from src to the
/// end of dest (the position of the first NUL in dest), then writes a final
/// NUL terminator after the appended bytes.
fn c_strncat(dest: &mut [u8], src: &[u8], n: usize) {
    let start = c_strlen(dest);
    let mut written = 0usize;
    while written < n {
        let pos = start + written;
        if pos >= dest.len() {
            return;
        }
        let c = if written < src.len() { src[written] } else { 0 };
        if c == 0 {
            dest[pos] = 0;
            return;
        }
        dest[pos] = c;
        written += 1;
    }
    let nul_pos = start + written;
    if nul_pos < dest.len() {
        dest[nul_pos] = 0;
    }
}

/// Process strings - main library entry point matching the C signature.
///
/// `input` is the input buffer (mutable in C, but we don't modify it).
/// `input_len` is the user-supplied input length.
/// `reference` is the reference buffer.
/// `ref_len` is the reference length.
pub fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // The C code checks `input == NULL`. In Rust, slices are non-null; we
    // treat an empty slice as "not null" since the caller always provides
    // a buffer.
    // (We intentionally do not return -1 because input is always non-null.)
    let _ = input;

    match operation {
        0 => {
            // Validate token - C also has `if (reference == NULL) return -2;`
            // but reference is always provided, so skip that.
            validate_token(input, reference)
        }
        1 => {
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
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            // Determine delimiter.
            let delim: u8 = if ref_len > 0 && !reference.is_empty() {
                reference[0]
            } else {
                b':'
            };
            find_delimiter(input, input_len, delim)
        }
        4 => {
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
                let next = if cmd_len < buffer.len() { buffer[cmd_len] } else { 0 };
                if next == 0 || next == b' ' {
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

fn compare_prefix(s: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match {
        if c_strcmp(s, prefix) == 0 {
            return 1;
        }
        let variations: [&[u8]; 5] = [b"_v1\0", b"_v2\0", b"_old\0", b"_new\0", b"_tmp\0"];
        for (i, var) in variations.iter().enumerate() {
            // char expected[64];
            let mut expected = [0u8; 64];
            // strncpy(expected, prefix, 63);
            c_strncpy(&mut expected[..63], prefix, 63);
            // expected[63] = '\0';
            expected[63] = 0;
            // strncat(expected, variations[i], 63 - strlen(expected));
            let cur_len = c_strlen(&expected);
            // The C code passes 63 - strlen(expected) as n (could underflow
            // if strlen(expected) > 63, but expected is limited).
            let n = 63usize.wrapping_sub(cur_len);
            c_strncat(&mut expected, var, n);
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

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        // Construct wildcard patterns using snprintf-like behavior.
        // wildcard_patterns[0] = "*<pattern>*"
        // wildcard_patterns[1] = "<pattern>*"
        // wildcard_patterns[2] = "*<pattern>"
        let mut wildcard_patterns: [[u8; 64]; 3] = [[0u8; 64]; 3];

        snprintf_wildcard(&mut wildcard_patterns[0], b"*", pattern, b"*");
        snprintf_wildcard(&mut wildcard_patterns[1], b"", pattern, b"*");
        snprintf_wildcard(&mut wildcard_patterns[2], b"*", pattern, b"");

        for i in 0..3 {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return 2 + i as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        if pattern_len == 0 {
            // strncmp with n=0 returns 0, so this would always match at i=0.
            // The loop runs i = 0..=text_len.
            // We replicate that behavior - return 10 + 0 = 10 for first iteration.
            // (Faithfully this would return 10 immediately.)
            return 10;
        }

        // C does `for (size_t i = 0; i <= text_len - pattern_len; i++)`.
        // If pattern_len > text_len, the subtraction underflows. The loop
        // would run effectively forever and read out of bounds. We avoid
        // crashing by checking the underflow condition first.
        if text_len >= pattern_len {
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
            let mut is_match = true;
            for i in 0..pattern_len {
                let mut c1 = if i < text.len() { text[i] } else { 0 };
                let mut c2 = if i < pattern.len() { pattern[i] } else { 0 };
                if c1 >= b'A' && c1 <= b'Z' {
                    c1 += 32;
                }
                if c2 >= b'A' && c2 <= b'Z' {
                    c2 += 32;
                }
                if c1 != c2 {
                    is_match = false;
                    break;
                }
            }
            if is_match {
                return 6;
            }
        }
    }

    0
}

/// Mimic C: snprintf(buf, 64, "%s%s%s", prefix, mid, suffix), which writes
/// up to 63 bytes plus a null terminator. If the result would exceed 63
/// bytes, it is truncated and null-terminated.
fn snprintf_wildcard(buf: &mut [u8; 64], prefix: &[u8], mid: &[u8], suffix: &[u8]) {
    let mut pos = 0usize;
    let max = 63usize; // leave space for NUL

    fn cstr<'a>(b: &'a [u8]) -> &'a [u8] {
        let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        &b[..n]
    }

    for src in [cstr(prefix), cstr(mid), cstr(suffix)].iter() {
        for &c in src.iter() {
            if pos >= max {
                break;
            }
            buf[pos] = c;
            pos += 1;
        }
        if pos >= max {
            break;
        }
    }
    if pos < buf.len() {
        buf[pos] = 0;
    } else {
        buf[buf.len() - 1] = 0;
    }
}
