//! Translation of `c_src/src/lib.c`.
//!
//! Memory model
//! ------------
//! `main` in `c_src/src/main.c` declares two 1024-byte `char` arrays and only
//! writes `input_len` / `ref_len` bytes into each.  Every comparison helper in
//! `lib.c` then calls `strlen` / `strcmp` / `strncmp` on them, so the reads run
//! straight off the end of the initialised prefix and, for a full buffer, off
//! the end of the array itself.  To reproduce that we model `main`'s whole stack
//! frame as one flat byte array ([`Frame`]) laid out exactly as the reference
//! binary lays it out (see [`crate::frame`]), and address the two buffers as
//! offsets into it.  A pointer is therefore an index, not a slice, and walking
//! past a buffer sees the next thing in the frame - which is what the C does.
//!
//! Reads that leave the modelled frame entirely have no meaningful value; the
//! only place the original code can get there is the containment scan in
//! `match_pattern`, where the reference program dies from `SIGSEGV`.  [`fault`]
//! reproduces that.

use crate::frame::{Frame, FRAME_LEN, INPUT_OFF, REF_OFF};

/// Reproduces the invalid read the C code performs when the containment scan in
/// `match_pattern` walks off the end of the stack: the reference program dies
/// from `SIGSEGV` before writing anything to stdout.
fn fault() -> ! {
    // Deliberate wild store, mirroring the out-of-bounds read in the C source.
    unsafe {
        core::ptr::write_volatile(0xdead_0000usize as *mut u8, 0);
    }
    // Only reached if the store above somehow did not trap.
    std::process::abort();
}

/// A NUL-terminated byte sequence, addressed the way C addresses a `char *`.
trait Bytes {
    fn at(&self, i: usize) -> u8;
}

/// A `char *` into `main`'s stack frame.
#[derive(Copy, Clone)]
struct P<'a> {
    f: &'a Frame,
    off: usize,
}

impl<'a> P<'a> {
    fn new(f: &'a Frame, off: usize) -> Self {
        P { f, off }
    }

    /// Pointer arithmetic (`p + n`).
    fn add(self, n: usize) -> Self {
        P {
            f: self.f,
            off: self.off.wrapping_add(n),
        }
    }
}

impl<'a> Bytes for P<'a> {
    fn at(&self, i: usize) -> u8 {
        let k = self.off.wrapping_add(i);
        if k >= FRAME_LEN {
            fault();
        }
        self.f.mem[k]
    }
}

/// A `char *` to one of `lib.c`'s own locals: a string literal, or a
/// `char[32]` / `char[64]` that the C code always NUL-terminates itself.  Bytes
/// past the end of the slice are the array's zero padding.
struct L<'a>(&'a [u8]);

impl<'a> Bytes for L<'a> {
    fn at(&self, i: usize) -> u8 {
        match self.0.get(i) {
            Some(&c) => c,
            None => 0,
        }
    }
}

/// `strlen`.
fn strlen<B: Bytes>(s: &B) -> usize {
    let mut i = 0usize;
    while s.at(i) != 0 {
        i += 1;
    }
    i
}

/// `strcmp`; returns the difference of the first differing `unsigned char`s.
fn strcmp<A: Bytes, B: Bytes>(a: &A, b: &B) -> i32 {
    let mut i = 0usize;
    loop {
        let ca = a.at(i);
        let cb = b.at(i);
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
fn strncmp<A: Bytes, B: Bytes>(a: &A, b: &B, n: usize) -> i32 {
    let mut i = 0usize;
    while i < n {
        let ca = a.at(i);
        let cb = b.at(i);
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

/// Main entrance function - performs various string comparison operations.
///
/// `frame` holds `main`'s locals; `input_len` and `ref_len` are the byte counts
/// `main` actually read into `input_buffer` and `ref_buffer`.
pub fn process_strings(
    frame: &Frame,
    input_len: usize,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    let input = P::new(frame, INPUT_OFF);
    let reference = P::new(frame, REF_OFF);

    // The C code checks `input == NULL` here; `main` always passes a stack
    // array, so the check can never fire.  The same applies to the
    // `reference == NULL` checks in cases 0, 2 and 4.

    match operation {
        0 => {
            /* Validate token */
            validate_token(input, reference)
        }

        1 => {
            /* Parse command from list - checks against multiple strings */
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input, input_len, &commands, 5)
        }

        2 => {
            /* Compare prefix - can use strcmp or strncmp based on flags */
            let exact = (flags & 0x01) as i32;
            compare_prefix(input, reference, exact)
        }

        3 => {
            /* Find delimiter position */
            let delim = if ref_len > 0 { reference.at(0) } else { b':' };
            find_delimiter(input, input_len, delim)
        }

        4 => {
            /* Match pattern */
            let case_sens = (flags & 0x02) as i32;
            match_pattern(input, reference, case_sens)
        }

        _ => -3,
    }
}

/// Validate token against expected value.
fn validate_token(token: P, expected: P) -> i32 {
    /* Direct strcmp */
    if strcmp(&token, &expected) == 0 {
        return 1; /* Valid */
    }

    /* Also check some common variations */
    if strcmp(&token, &L(b"VALID")) == 0 || strcmp(&token, &L(b"OK")) == 0 {
        return 1;
    }

    0 /* Invalid */
}

/// Parse command from a list of valid commands.
fn parse_command(buffer: P, buf_size: usize, cmd_list: &[&[u8]], list_size: i32) -> i32 {
    /* Iterate through command list */
    for i in 0..list_size {
        let cmd = L(cmd_list[i as usize]);
        /* Safe comparison using strncmp first */
        let cmd_len = strlen(&cmd);

        if buf_size >= cmd_len {
            if strncmp(&buffer, &cmd, cmd_len) == 0 {
                /* Check if exact match */
                let next = buffer.at(cmd_len);
                if next == b'\0' || next == b' ' {
                    return i; /* Return command index */
                }
            }
        }

        /* Fallback: direct strcmp */
        if strcmp(&buffer, &cmd) == 0 {
            return i;
        }
    }

    /* Check for special admin command */
    if strcmp(&buffer, &L(b"ADMIN")) == 0 {
        return 99;
    }

    -1 /* No match */
}

/// Compare prefix with optional exact matching.
fn compare_prefix(s: P, prefix: P, exact_match: i32) -> i32 {
    let prefix_len = strlen(&prefix);

    if exact_match != 0 {
        /* Exact match required */
        if strcmp(&s, &prefix) == 0 {
            return 1;
        }

        /* Try with some common suffixes */
        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for i in 0..5usize {
            /* Construct expected string: `char expected[64]` */
            let mut expected = [0u8; 64];

            // strncpy(expected, prefix, 63): copies at most 63 bytes and pads
            // the remainder of those 63 with NUL.
            let copy = if prefix_len < 63 { prefix_len } else { 63 };
            for j in 0..copy {
                expected[j] = prefix.at(j);
            }
            expected[63] = 0;

            // strncat(expected, variations[i], 63 - strlen(expected))
            let cur = strlen(&L(&expected));
            let room = 63 - cur;
            let var = variations[i];
            let var_len = var.len();
            let append = if var_len < room { var_len } else { room };
            expected[cur..cur + append].copy_from_slice(&var[..append]);
            expected[cur + append] = 0;

            if strcmp(&s, &L(&expected)) == 0 {
                return 2 + i as i32;
            }
        }

        0
    } else {
        /* Prefix match only - safer with strncmp */
        if strncmp(&s, &prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

/// Find delimiter position in string.
fn find_delimiter(data: P, len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    /* Manual search with bounds checking */
    for i in 0..len {
        let c = data.at(i);
        if c == delim {
            return i as i32;
        }
        if c == 0 {
            break;
        }
    }

    /* Check for special delimiter patterns using strcmp */
    if delim == b'|' && strcmp(&data, &L(b"NONE")) == 0 {
        return -2; /* Special case */
    }

    if delim == b':' && strcmp(&data, &L(b"EMPTY")) == 0 {
        return -3; /* Special case */
    }

    -1 /* Not found */
}

/// Match pattern with optional case sensitivity.
fn match_pattern(text: P, pattern: P, case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        /* Case-sensitive exact match */
        if strcmp(&text, &pattern) == 0 {
            return 1;
        }

        /* Try with wildcards - construct patterns */
        let wildcard_patterns: [Vec<u8>; 3] = [
            snprintf_wrap(b"*", &pattern, b"*"),
            snprintf_wrap(b"", &pattern, b"*"),
            snprintf_wrap(b"*", &pattern, b""),
        ];

        for i in 0..3usize {
            if strcmp(&text, &L(&wildcard_patterns[i])) == 0 {
                return 2 + i as i32;
            }
        }

        /* Check if text contains pattern */
        let text_len = strlen(&text);
        let pattern_len = strlen(&pattern);

        // `text_len - pattern_len` is `size_t` arithmetic and wraps around when
        // the pattern is longer than the text; the C loop then scans far past
        // the end of the stack frame and the process dies.
        let bound = text_len.wrapping_sub(pattern_len);
        let mut i = 0usize;
        while i <= bound {
            if strncmp(&text.add(i), &pattern, pattern_len) == 0 {
                return 10usize.wrapping_add(i) as i32; /* Return position + offset */
            }
            i += 1;
        }
    } else {
        /* Case-insensitive - need to check both cases */
        /* First try exact match */
        if strcmp(&text, &pattern) == 0 {
            return 1;
        }

        /* Manual case-insensitive comparison */
        let pattern_len = strlen(&pattern);
        let text_len = strlen(&text);

        if text_len != pattern_len {
            /* Try prefix match with strncmp */
            if strncmp(&text, &pattern, pattern_len) == 0 {
                return 5;
            }
        }

        /* Compare character by character (safer) */
        if text_len == pattern_len {
            let mut matched = 1;
            for i in 0..pattern_len {
                let mut c1 = text.at(i);
                let mut c2 = pattern.at(i);

                /* Convert to lowercase */
                if c1 >= b'A' && c1 <= b'Z' {
                    c1 = c1.wrapping_add(32);
                }
                if c2 >= b'A' && c2 <= b'Z' {
                    c2 = c2.wrapping_add(32);
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

/// `snprintf(dst, 64, "<pre>%s<post>", pattern)` into a `char[64]`: at most 63
/// characters are stored, followed by the NUL terminator.
fn snprintf_wrap<B: Bytes>(pre: &[u8], pattern: &B, post: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(pre);
    let mut i = 0usize;
    loop {
        let c = pattern.at(i);
        if c == 0 {
            break;
        }
        out.push(c);
        i += 1;
    }
    out.extend_from_slice(post);
    out.truncate(63);
    out
}
