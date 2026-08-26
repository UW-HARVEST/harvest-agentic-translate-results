// Translated from c_src/src/lib.c
// Reproduces the original C semantics exactly, including unsafe behavior.

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};

// Declare C standard library functions for byte-identical behavior.
extern "C" {
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strncat(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
}

/// process_strings - Main entry point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_strings(
    input: *mut c_char,
    input_len: usize,
    reference: *const c_char,
    ref_len: usize,
    operation: c_int,
    flags: u32,
) -> c_int {
    if input.is_null() {
        return -1;
    }

    match operation {
        0 => {
            // Validate token - VULNERABLE if input not null-terminated
            if reference.is_null() {
                return -2;
            }
            validate_token(input, reference)
        }
        1 => {
            // Parse command from list - checks against multiple strings
            // Build the command list as C strings.
            let cmd0 = b"START\0".as_ptr() as *const c_char;
            let cmd1 = b"STOP\0".as_ptr() as *const c_char;
            let cmd2 = b"PAUSE\0".as_ptr() as *const c_char;
            let cmd3 = b"RESUME\0".as_ptr() as *const c_char;
            let cmd4 = b"RESET\0".as_ptr() as *const c_char;
            let commands: [*const c_char; 5] = [cmd0, cmd1, cmd2, cmd3, cmd4];
            parse_command(input, input_len, commands.as_ptr(), 5)
        }
        2 => {
            // Compare prefix - can use strcmp or strncmp based on flags
            if reference.is_null() {
                return -2;
            }
            let exact: c_int = (flags & 0x01) as c_int;
            compare_prefix(input, reference, exact)
        }
        3 => {
            // Find delimiter position
            let delim: c_char = if !reference.is_null() && ref_len > 0 {
                *reference
            } else {
                b':' as c_char
            };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            // Match pattern - VULNERABLE in certain paths
            if reference.is_null() {
                return -2;
            }
            let case_sens: c_int = (flags & 0x02) as c_int;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

/// Validate token against expected value.
unsafe fn validate_token(token: *const c_char, expected: *const c_char) -> c_int {
    if strcmp(token, expected) == 0 {
        return 1;
    }

    let valid = b"VALID\0".as_ptr() as *const c_char;
    let ok = b"OK\0".as_ptr() as *const c_char;

    if strcmp(token, valid) == 0 || strcmp(token, ok) == 0 {
        return 1;
    }

    0
}

/// Parse command from a list of valid commands.
unsafe fn parse_command(
    buffer: *mut c_char,
    buf_size: usize,
    cmd_list: *const *const c_char,
    list_size: c_int,
) -> c_int {
    let mut i: c_int = 0;
    while i < list_size {
        let cmd = *cmd_list.offset(i as isize);
        let cmd_len = strlen(cmd);

        if buf_size >= cmd_len {
            if strncmp(buffer as *const c_char, cmd, cmd_len) == 0 {
                let next = *buffer.add(cmd_len);
                if next == 0 || next == b' ' as c_char {
                    return i;
                }
            }
        }

        if strcmp(buffer as *const c_char, cmd) == 0 {
            return i;
        }

        i += 1;
    }

    let admin = b"ADMIN\0".as_ptr() as *const c_char;
    if strcmp(buffer as *const c_char, admin) == 0 {
        return 99;
    }

    -1
}

/// Compare prefix with optional exact matching.
unsafe fn compare_prefix(str_: *const c_char, prefix: *const c_char, exact_match: c_int) -> c_int {
    let prefix_len = strlen(prefix);

    if exact_match != 0 {
        if strcmp(str_, prefix) == 0 {
            return 1;
        }

        // char variations[5][32] = {"_v1", "_v2", "_old", "_new", "_tmp"};
        // Replicate the layout: 5 rows of 32 bytes each, zero-initialized,
        // first bytes set to the suffix string.
        let mut variations = [[0i8; 32]; 5];
        let suffixes: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (row, sfx) in variations.iter_mut().zip(suffixes.iter()) {
            for (j, &b) in sfx.iter().enumerate() {
                row[j] = b as i8;
            }
        }

        for i in 0..5usize {
            // char expected[64];
            let mut expected = [0i8; 64];
            // strncpy(expected, prefix, 63);
            strncpy(
                expected.as_mut_ptr() as *mut c_char,
                prefix,
                63,
            );
            // expected[63] = '\0';
            expected[63] = 0;
            // strncat(expected, variations[i], 63 - strlen(expected));
            let cur_len = strlen(expected.as_ptr() as *const c_char);
            let n = 63usize.wrapping_sub(cur_len);
            strncat(
                expected.as_mut_ptr() as *mut c_char,
                variations[i].as_ptr() as *const c_char,
                n,
            );

            if strcmp(str_, expected.as_ptr() as *const c_char) == 0 {
                return (2 + i) as c_int;
            }
        }

        0
    } else {
        if strncmp(str_, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

/// Find delimiter position in string.
unsafe fn find_delimiter(data: *const c_char, len: usize, delim: c_char) -> c_int {
    if len == 0 {
        return -1;
    }

    let mut i: usize = 0;
    while i < len {
        let b = *data.add(i);
        if b == delim {
            return i as c_int;
        }
        if b == 0 {
            break;
        }
        i += 1;
    }

    let none = b"NONE\0".as_ptr() as *const c_char;
    let empty = b"EMPTY\0".as_ptr() as *const c_char;

    if delim == b'|' as c_char && strcmp(data, none) == 0 {
        return -2;
    }

    if delim == b':' as c_char && strcmp(data, empty) == 0 {
        return -3;
    }

    -1
}

/// Match pattern with optional case sensitivity.
unsafe fn match_pattern(text: *const c_char, pattern: *const c_char, case_sensitive: c_int) -> c_int {
    if case_sensitive != 0 {
        if strcmp(text, pattern) == 0 {
            return 1;
        }

        // char wildcard_patterns[3][64];
        let mut wildcard_patterns = [[0i8; 64]; 3];
        // snprintf(wildcard_patterns[0], 64, "*%s*", pattern);
        snprintf(
            wildcard_patterns[0].as_mut_ptr() as *mut c_char,
            64,
            b"*%s*\0".as_ptr() as *const c_char,
            pattern,
        );
        // snprintf(wildcard_patterns[1], 64, "%s*", pattern);
        snprintf(
            wildcard_patterns[1].as_mut_ptr() as *mut c_char,
            64,
            b"%s*\0".as_ptr() as *const c_char,
            pattern,
        );
        // snprintf(wildcard_patterns[2], 64, "*%s", pattern);
        snprintf(
            wildcard_patterns[2].as_mut_ptr() as *mut c_char,
            64,
            b"*%s\0".as_ptr() as *const c_char,
            pattern,
        );

        for i in 0..3usize {
            if strcmp(text, wildcard_patterns[i].as_ptr() as *const c_char) == 0 {
                return (2 + i) as c_int;
            }
        }

        let text_len = strlen(text);
        let pattern_len = strlen(pattern);

        // for (size_t i = 0; i <= text_len - pattern_len; i++)
        // text_len - pattern_len uses size_t (wrapping) semantics in C.
        let limit: usize = text_len.wrapping_sub(pattern_len);
        let mut i: usize = 0;
        while i <= limit {
            if strncmp(text.add(i), pattern, pattern_len) == 0 {
                return (10usize + i) as c_int;
            }
            i = i.wrapping_add(1);
            // Guard against runaway loop in pathological inputs that wouldn't
            // happen in the C version due to UB; but to match faithfully,
            // we shouldn't add a guard. C also doesn't guard. Remove guard.
            // (Keeping the loop unbounded matches the C source.)
            if i == 0 {
                break; // wrapped around (shouldn't happen normally)
            }
        }

        0
    } else {
        if strcmp(text, pattern) == 0 {
            return 1;
        }

        let pattern_len = strlen(pattern);
        let text_len = strlen(text);

        if text_len != pattern_len {
            if strncmp(text, pattern, pattern_len) == 0 {
                return 5;
            }
        }

        if text_len == pattern_len {
            let mut matched: c_int = 1;
            let mut i: usize = 0;
            while i < pattern_len {
                let mut c1 = *text.add(i) as u8;
                let mut c2 = *pattern.add(i) as u8;

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
                i += 1;
            }
            if matched != 0 {
                return 6;
            }
        }

        0
    }
}

// Suppress unused warning for memset import (kept for potential future use).
#[allow(dead_code)]
fn _unused_memset_link() {
    let _ = memset as unsafe extern "C" fn(*mut c_void, c_int, usize) -> *mut c_void;
}
