//! Translation of `c_src/src/lib.c`.
//!
//! Memory model
//! ------------
//! The C driver hands `process_strings` two 1024-byte stack buffers of which
//! only the first `input_len` / `ref_len` bytes are initialised.  Several of the
//! comparison helpers below deliberately walk past those lengths (the C code
//! calls `strcmp`/`strlen` on buffers that are not guaranteed to be
//! NUL-terminated).  Here the buffers are modelled as 1024 zero-filled bytes,
//! i.e. reads past the initialised prefix observe `\0`, which is what a
//! well-formed input relies on.  Reads past the *end* of the 1024-byte array
//! have no meaningful model at all; the one place where the original code can
//! deterministically reach that point (the containment scan in
//! `match_pattern`) faults, exactly like the C program does.

/// Size of the buffers handed in by `main` (`MAX_BUFFER_SIZE`).
pub const BUF_SIZE: usize = 1024;

/// `strlen` over a byte region that is implicitly NUL-terminated at its end.
fn c_strlen(s: &[u8]) -> usize {
    match s.iter().position(|&c| c == 0) {
        Some(i) => i,
        None => s.len(),
    }
}

/// Byte at `i`, with the region behind the slice reading as `\0`.
fn at(s: &[u8], i: usize) -> u8 {
    match s.get(i) {
        Some(&c) => c,
        None => 0,
    }
}

/// `strcmp`; returns the difference of the first differing `unsigned char`s.
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = at(a, i);
        let cb = at(b, i);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

/// `strncmp`.
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = at(a, i);
        let cb = at(b, i);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// Reproduces the invalid read the C code performs when the containment scan in
/// `match_pattern` runs off the end of the caller's buffer: the reference
/// program dies from SIGSEGV before writing anything to stdout.
fn fault() -> ! {
    // Deliberate wild store, mirroring the out-of-bounds read in the C source.
    unsafe {
        core::ptr::write_volatile(0xdead_0000usize as *mut u8, 0);
    }
    // Only reached if the store above somehow did not trap.
    std::process::exit(139);
}

/// Main entrance function - performs various string comparison operations.
///
/// `input` and `reference` are the full 1024-byte buffers; `input_len` and
/// `ref_len` are the counts of bytes that `main` actually filled in.
pub fn process_strings(
    input: &[u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // The C code checks `input == NULL` here; `main` always passes a stack
    // array, so the check can never fire.

    match operation {
        0 => {
            /* Validate token */
            // `reference == NULL` check can never fire either.
            validate_token(input, reference)
        }

        1 => {
            /* Parse command from list - checks against multiple strings */
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input, input_len, &commands, 5)
        }

        2 => {
            /* Compare prefix - can use strcmp or strncmp based on flags */
            let exact = flags & 0x01;
            compare_prefix(input, reference, exact as i32)
        }

        3 => {
            /* Find delimiter position */
            let delim = if ref_len > 0 { at(reference, 0) } else { b':' };
            find_delimiter(input, input_len, delim)
        }

        4 => {
            /* Match pattern */
            let case_sens = flags & 0x02;
            match_pattern(input, reference, case_sens as i32)
        }

        _ => -3,
    }
}

/// Validate token against expected value.
fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1; /* Valid */
    }

    /* Also check some common variations */
    if c_strcmp(token, b"VALID") == 0 || c_strcmp(token, b"OK") == 0 {
        return 1;
    }

    0 /* Invalid */
}

/// Parse command from a list of valid commands.
fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]], list_size: i32) -> i32 {
    /* Iterate through command list */
    for i in 0..list_size {
        let cmd = cmd_list[i as usize];
        /* Safe comparison using strncmp first */
        let cmd_len = c_strlen(cmd);

        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd, cmd_len) == 0 {
                /* Check if exact match */
                let next = at(buffer, cmd_len);
                if next == b'\0' || next == b' ' {
                    return i; /* Return command index */
                }
            }
        }

        /* Fallback: direct strcmp */
        if c_strcmp(buffer, cmd) == 0 {
            return i;
        }
    }

    /* Check for special admin command */
    if c_strcmp(buffer, b"ADMIN") == 0 {
        return 99;
    }

    -1 /* No match */
}

/// Compare prefix with optional exact matching.
fn compare_prefix(s: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match != 0 {
        /* Exact match required */
        if c_strcmp(s, prefix) == 0 {
            return 1;
        }

        /* Try with some common suffixes */
        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for i in 0..5usize {
            /* Construct expected string */
            let mut expected = [0u8; 64];

            // strncpy(expected, prefix, 63): copies up to 63 bytes, zero padded.
            let copy = if prefix_len < 63 { prefix_len } else { 63 };
            for j in 0..copy {
                expected[j] = at(prefix, j);
            }
            expected[63] = 0;

            // strncat(expected, variations[i], 63 - strlen(expected))
            let cur = c_strlen(&expected);
            let room = 63 - cur;
            let var = variations[i];
            let var_len = c_strlen(var);
            let append = if var_len < room { var_len } else { room };
            for j in 0..append {
                expected[cur + j] = var[j];
            }
            expected[cur + append] = 0;

            if c_strcmp(s, &expected) == 0 {
                return 2 + i as i32;
            }
        }

        0
    } else {
        /* Prefix match only */
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

    /* Manual search with bounds checking */
    for i in 0..len {
        let c = at(data, i);
        if c == delim {
            return i as i32;
        }
        if c == 0 {
            break;
        }
    }

    /* Check for special delimiter patterns using strcmp */
    if delim == b'|' && c_strcmp(data, b"NONE") == 0 {
        return -2; /* Special case */
    }

    if delim == b':' && c_strcmp(data, b"EMPTY") == 0 {
        return -3; /* Special case */
    }

    -1 /* Not found */
}

/// Match pattern with optional case sensitivity.
fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        /* Case-sensitive exact match */
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        /* Try with wildcards - construct patterns */
        let pat = &pattern[..c_strlen(pattern)];
        let mut wildcard_patterns: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        wildcard_patterns[0] = snprintf_wrap(b"*", pat, b"*");
        wildcard_patterns[1] = snprintf_wrap(b"", pat, b"*");
        wildcard_patterns[2] = snprintf_wrap(b"*", pat, b"");

        for i in 0..3usize {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return 2 + i as i32;
            }
        }

        /* Check if text contains pattern */
        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        // `text_len - pattern_len` is computed in size_t and wraps around when
        // the pattern is longer than the text; the C loop then scans far past
        // the end of the buffer.
        let bound = text_len.wrapping_sub(pattern_len);
        let mut i = 0usize;
        while i <= bound {
            if i > text.len() {
                // Past the caller's array: the C program faults here.
                fault();
            }
            if c_strncmp(&text[i.min(text.len())..], pattern, pattern_len) == 0 {
                return (10usize.wrapping_add(i)) as i32; /* Return position + offset */
            }
            i += 1;
        }
    } else {
        /* Case-insensitive - need to check both cases */
        /* First try exact match */
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        /* Manual case-insensitive comparison */
        let pattern_len = c_strlen(pattern);
        let text_len = c_strlen(text);

        if text_len != pattern_len {
            /* Try prefix match with strncmp */
            if c_strncmp(text, pattern, pattern_len) == 0 {
                return 5;
            }
        }

        /* Compare character by character */
        if text_len == pattern_len {
            let mut matched = true;
            for i in 0..pattern_len {
                let mut c1 = at(text, i);
                let mut c2 = at(pattern, i);

                /* Convert to lowercase */
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

    0 /* No match */
}

/// `snprintf(dst, 64, "<pre>%s<post>", pattern)`: the result is truncated to 63
/// bytes plus the NUL terminator.
fn snprintf_wrap(pre: &[u8], pattern: &[u8], post: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(pre);
    out.extend_from_slice(pattern);
    out.extend_from_slice(post);
    out.truncate(63);
    out
}
