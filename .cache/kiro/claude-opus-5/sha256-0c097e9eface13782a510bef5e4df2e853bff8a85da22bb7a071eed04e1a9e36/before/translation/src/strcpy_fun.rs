/*
 * Rust translation of c_src/src/lib.c
 *
 * Behaviour-preserving port. The C code deliberately calls strcmp()/strncmp()
 * on buffers that may not be NUL terminated (the original comments flag these
 * as "VULNERABLE"). Those bugs are *not* fixed here: they are reproduced by
 * modelling the two 1024 byte stack buffers of main() as zero-initialised byte
 * arrays with a small zero pad, so a "string" simply ends at the first zero
 * byte, exactly as strcmp() would see it.
 */

/* Size of the caller's buffer (MAX_BUFFER_SIZE in main.c). */
pub const MAX_BUFFER_SIZE: usize = 1024;

/* ------------------------------------------------------------------ */
/* C string helpers over a byte buffer                                 */
/* ------------------------------------------------------------------ */

/// `strlen(&buf[off])` - number of bytes before the first NUL.
fn strlen_at(buf: &[u8], off: usize) -> usize {
    let mut i = off;
    while i < buf.len() && buf[i] != 0 {
        i += 1;
    }
    i - off
}

/// The C string starting at `buf[off]`, i.e. bytes up to (not including) NUL.
fn cstr_at(buf: &[u8], off: usize) -> &[u8] {
    let start = off.min(buf.len());
    let n = strlen_at(buf, start);
    &buf[start..start + n]
}

/// The C string starting at `buf[0]`.
fn cstr(buf: &[u8]) -> &[u8] {
    cstr_at(buf, 0)
}

/// `strcmp(a, b) == 0` where both arguments are already NUL trimmed.
fn strcmp_eq(a: &[u8], b: &[u8]) -> bool {
    a == b
}

/// `strncmp(a, b, n) == 0`. `a`/`b` are raw (possibly NUL containing) byte
/// slices; bytes past the end of a slice read as 0 (zero-initialised memory).
fn strncmp_eq(a: &[u8], b: &[u8], n: usize) -> bool {
    for i in 0..n {
        let ca = a.get(i).copied().unwrap_or(0);
        let cb = b.get(i).copied().unwrap_or(0);
        if ca != cb {
            return false;
        }
        if ca == 0 {
            /* both NUL: strings are equal up to here */
            return true;
        }
    }
    true
}

/// `snprintf(dst, cap, ...)` with the concatenation of `parts`: the result is
/// truncated to `cap - 1` bytes (the NUL always fits).
fn snprintf_concat(parts: &[&[u8]], cap: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for p in parts {
        out.extend_from_slice(p);
    }
    if out.len() > cap - 1 {
        out.truncate(cap - 1);
    }
    out
}

/// The C program walks off the end of its stack buffers here and dies with
/// SIGSEGV. Reproduce the crash rather than inventing a return value.
fn c_out_of_bounds_read() -> ! {
    // SAFETY: intentional - emulates the out-of-bounds read of the C original,
    // which faults. read_volatile keeps the access from being optimised away.
    unsafe {
        let _ = std::ptr::read_volatile(std::ptr::null::<u8>());
    }
    std::process::abort();
}

/* ------------------------------------------------------------------ */
/* Main entrance function                                              */
/* ------------------------------------------------------------------ */

/// Port of `process_strings()`.
///
/// `input` / `reference` model the C `char *` arguments; `None` models NULL.
pub fn process_strings(
    input: Option<&[u8]>,
    input_len: usize,
    reference: Option<&[u8]>,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    let input = match input {
        None => return -1,
        Some(p) => p,
    };

    /* Different operations based on operation code */
    match operation {
        0 => {
            /* Validate token */
            let reference = match reference {
                None => return -2,
                Some(p) => p,
            };
            validate_token(input, reference)
        }

        1 => {
            /* Parse command from list - checks against multiple strings */
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input, input_len, &commands, 5)
        }

        2 => {
            /* Compare prefix - can use strcmp or strncmp based on flags */
            let reference = match reference {
                None => return -2,
                Some(p) => p,
            };
            let exact = flags & 0x01;
            compare_prefix(input, reference, exact as i32)
        }

        3 => {
            /* Find delimiter position */
            let delim: u8 = match reference {
                Some(r) if ref_len > 0 => r[0],
                _ => b':',
            };
            find_delimiter(input, input_len, delim)
        }

        4 => {
            /* Match pattern */
            let reference = match reference {
                None => return -2,
                Some(p) => p,
            };
            let case_sens = flags & 0x02;
            match_pattern(input, reference, case_sens as i32)
        }

        _ => -3,
    }
}

/* ------------------------------------------------------------------ */

/// Port of `validate_token()`.
fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    /* Direct strcmp */
    if strcmp_eq(cstr(token), cstr(expected)) {
        return 1; /* Valid */
    }

    /* Also check some common variations */
    if strcmp_eq(cstr(token), b"VALID") || strcmp_eq(cstr(token), b"OK") {
        return 1;
    }

    0 /* Invalid */
}

/// Port of `parse_command()`.
fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]], list_size: i32) -> i32 {
    /* Iterate through command list */
    for i in 0..list_size {
        let cmd = cmd_list[i as usize];
        /* Safe comparison using strncmp first */
        let cmd_len = cmd.len(); /* strlen(cmd_list[i]) */

        if buf_size >= cmd_len {
            if strncmp_eq(buffer, cmd, cmd_len) {
                /* Check if exact match */
                let c = buffer.get(cmd_len).copied().unwrap_or(0);
                if c == 0 || c == b' ' {
                    return i; /* Return command index */
                }
            }
        }

        /* Fallback: direct strcmp */
        if strcmp_eq(cstr(buffer), cmd) {
            return i;
        }
    }

    /* Check for special admin command */
    if strcmp_eq(cstr(buffer), b"ADMIN") {
        return 99;
    }

    -1 /* No match */
}

/// Port of `compare_prefix()`.
fn compare_prefix(str_: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = strlen_at(prefix, 0);

    if exact_match != 0 {
        /* Exact match required */
        if strcmp_eq(cstr(str_), cstr(prefix)) {
            return 1;
        }

        /* Try with some common suffixes */
        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for i in 0..5usize {
            /* Construct expected string:
             *   char expected[64];
             *   strncpy(expected, prefix, 63);   -- NUL pads to 63 bytes
             *   expected[63] = '\0';
             *   strncat(expected, variations[i], 63 - strlen(expected));
             */
            let mut expected = [0u8; 64];
            let src = cstr(prefix);
            let n = src.len().min(63);
            expected[..n].copy_from_slice(&src[..n]);
            /* remaining bytes are already 0, and expected[63] = '\0' */

            let cur_len = strlen_at(&expected, 0); /* == n */
            let room = 63 - cur_len;
            let add = variations[i].len().min(room);
            expected[cur_len..cur_len + add].copy_from_slice(&variations[i][..add]);
            expected[cur_len + add] = 0;

            if strcmp_eq(cstr(str_), cstr(&expected)) {
                return 2 + i as i32;
            }
        }

        0
    } else {
        /* Prefix match only - safer with strncmp */
        if strncmp_eq(str_, prefix, prefix_len) {
            return 1;
        }
        0
    }
}

/// Port of `find_delimiter()`.
fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    /* Manual search with bounds checking */
    for i in 0..len {
        let b = data.get(i).copied().unwrap_or(0);
        if b == delim {
            return i as i32;
        }
        if b == 0 {
            break;
        }
    }

    /* Check for special delimiter patterns using strcmp */
    if delim == b'|' && strcmp_eq(cstr(data), b"NONE") {
        return -2; /* Special case */
    }

    if delim == b':' && strcmp_eq(cstr(data), b"EMPTY") {
        return -3; /* Special case */
    }

    -1 /* Not found */
}

/// Port of `match_pattern()`.
fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        /* Case-sensitive exact match */
        if strcmp_eq(cstr(text), cstr(pattern)) {
            return 1;
        }

        /* Try with wildcards - construct patterns (snprintf, 64 byte bufs) */
        let p = cstr(pattern);
        let wildcard_patterns: [Vec<u8>; 3] = [
            snprintf_concat(&[b"*", p, b"*"], 64),
            snprintf_concat(&[p, b"*"], 64),
            snprintf_concat(&[b"*", p], 64),
        ];

        for i in 0..3usize {
            if strcmp_eq(cstr(text), &wildcard_patterns[i]) {
                return 2 + i as i32;
            }
        }

        /* Check if text contains pattern.
         *
         * The C loop bound is `text_len - pattern_len` computed in size_t: when
         * pattern_len > text_len it underflows to a huge value and the loop runs
         * off the end of the buffer. That bug is preserved. */
        let text_len = strlen_at(text, 0);
        let pattern_len = strlen_at(pattern, 0);

        let bound = text_len.wrapping_sub(pattern_len);
        let mut i: usize = 0;
        loop {
            if i > bound {
                break;
            }
            if i >= text.len() {
                /* past the end of the C stack buffer: the original faults */
                c_out_of_bounds_read();
            }
            if strncmp_eq(&text[i..], pattern, pattern_len) {
                return 10 + i as i32; /* Return position + offset */
            }
            i += 1;
        }
    } else {
        /* Case-insensitive - first try exact match */
        if strcmp_eq(cstr(text), cstr(pattern)) {
            return 1;
        }

        /* Manual case-insensitive comparison */
        let pattern_len = strlen_at(pattern, 0);
        let text_len = strlen_at(text, 0);

        if text_len != pattern_len {
            /* Try prefix match with strncmp */
            if strncmp_eq(text, pattern, pattern_len) {
                return 5;
            }
        }

        /* Compare character by character (safer) */
        if text_len == pattern_len {
            let mut matched = 1;
            for i in 0..pattern_len {
                let mut c1 = text[i];
                let mut c2 = pattern[i];

                /* Convert to lowercase */
                if c1 >= b'A' && c1 <= b'Z' {
                    c1 += 32;
                }
                if c2 >= b'A' && c2 <= b'Z' {
                    c2 += 32;
                }

                if c1 != c2 {
                    matched = 0;
                    break;
                }
            }
            if matched != 0 {
                return 6;
            }
        }
    }

    0 /* No match */
}
