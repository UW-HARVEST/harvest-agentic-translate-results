//! Translation of `c_src/src/lib.c`.
//!
//! The C code operates on raw `char *` pointers into the two 1024 byte stack
//! buffers of `main()`.  Here those buffers are modelled as byte slices; a
//! slice always reaches to the end of the modelled buffer so that the C string
//! routines can walk past the "written" region exactly like the C code does.

use crate::cstr::{
    segfault, slice_from, snprintf_concat, strcmp, strlen, strncat, strncmp, strncpy,
};

/// Main entrance function - performs various string comparison operations
///
/// * `input`      - input string buffer (`None` models a NULL pointer)
/// * `input_len`  - length of input buffer
/// * `reference`  - reference string for comparison (`None` models NULL)
/// * `ref_len`    - length of reference buffer
/// * `operation`  - operation code (0..4)
/// * `flags`      - operation flags (case sensitivity, exact match, etc)
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
        Some(input) => input,
    };

    /* Different operations based on operation code */
    match operation {
        0 => {
            /* Validate token - VULNERABLE if input not null-terminated */
            let reference = match reference {
                None => return -2,
                Some(r) => r,
            };
            validate_token(input, reference)
        }

        1 => {
            /* Parse command from list - checks against multiple strings */
            let commands: [&[u8]; 5] = [b"START\0", b"STOP\0", b"PAUSE\0", b"RESUME\0", b"RESET\0"];
            parse_command(input, input_len, &commands, 5)
        }

        2 => {
            /* Compare prefix - can use strcmp or strncmp based on flags */
            let reference = match reference {
                None => return -2,
                Some(r) => r,
            };
            let exact = (flags & 0x01) as i32;
            compare_prefix(input, reference, exact)
        }

        3 => {
            /* Find delimiter position */
            let delim = match reference {
                Some(r) if ref_len > 0 => r[0],
                _ => b':',
            };
            find_delimiter(input, input_len, delim)
        }

        4 => {
            /* Match pattern - VULNERABLE in certain paths */
            let reference = match reference {
                None => return -2,
                Some(r) => r,
            };
            let case_sens = (flags & 0x02) as i32;
            match_pattern(input, reference, case_sens)
        }

        _ => -3,
    }
}

/// Validate token against expected value
/// VULNERABLE: Uses strcmp without ensuring null termination
fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    /* Direct strcmp - will overflow if token not null-terminated */
    if strcmp(token, expected) == 0 {
        return 1; /* Valid */
    }

    /* Also check some common variations */
    if strcmp(token, b"VALID\0") == 0 || strcmp(token, b"OK\0") == 0 {
        return 1;
    }

    0 /* Invalid */
}

/// Parse command from a list of valid commands
/// Mix of safe and unsafe comparisons
fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]], list_size: i32) -> i32 {
    /* Iterate through command list */
    for i in 0..list_size {
        /* Safe comparison using strncmp first */
        let cmd = cmd_list[i as usize];
        let cmd_len = strlen(cmd);

        if buf_size >= cmd_len {
            if strncmp(buffer, cmd, cmd_len) == 0 {
                /* Check if exact match - VULNERABLE if buffer not null-terminated */
                let c = match buffer.get(cmd_len) {
                    Some(&c) => c,
                    None => segfault(),
                };
                if c == b'\0' || c == b' ' {
                    return i; /* Return command index */
                }
            }
        }

        /* Fallback: direct strcmp - VULNERABLE */
        if strcmp(buffer, cmd) == 0 {
            return i;
        }
    }

    /* Check for special admin command - always vulnerable strcmp */
    if strcmp(buffer, b"ADMIN\0") == 0 {
        return 99;
    }

    -1 /* No match */
}

/// Compare prefix with optional exact matching
/// Safe when exact_match=0, vulnerable when exact_match=1
fn compare_prefix(str_: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = strlen(prefix);

    if exact_match != 0 {
        /* Exact match required - VULNERABLE strcmp */
        if strcmp(str_, prefix) == 0 {
            return 1;
        }

        /* Try with some common suffixes */
        let mut variations = [[0u8; 32]; 5];
        for (row, text) in variations
            .iter_mut()
            .zip([b"_v1".as_slice(), b"_v2", b"_old", b"_new", b"_tmp"])
        {
            row[..text.len()].copy_from_slice(text);
        }

        for i in 0..5usize {
            /* Construct expected string */
            let mut expected = [0u8; 64];
            strncpy(&mut expected, prefix, 63);
            expected[63] = 0;
            let dlen = strlen(&expected);
            let variation = variations[i];
            strncat(&mut expected, &variation, 63 - dlen);

            /* VULNERABLE: strcmp without length check on str */
            if strcmp(str_, &expected) == 0 {
                return 2 + i as i32;
            }
        }

        0
    } else {
        /* Prefix match only - safer with strncmp */
        if strncmp(str_, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

/// Find delimiter position in string
/// Uses strncmp for safety but has edge cases
fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    /* Manual search with bounds checking */
    for i in 0..len {
        let b = match data.get(i) {
            Some(&b) => b,
            None => segfault(),
        };
        if b == delim {
            return i as i32;
        }
        if b == 0 {
            break;
        }
    }

    /* Check for special delimiter patterns using strcmp */
    /* VULNERABLE: doesn't verify data is null-terminated */
    if delim == b'|' && strcmp(data, b"NONE\0") == 0 {
        return -2; /* Special case */
    }

    if delim == b':' && strcmp(data, b"EMPTY\0") == 0 {
        return -3; /* Special case */
    }

    -1 /* Not found */
}

/// Match pattern with optional case sensitivity
/// Multiple vulnerable strcmp calls
fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        /* Case-sensitive exact match - VULNERABLE */
        if strcmp(text, pattern) == 0 {
            return 1;
        }

        /* Try with wildcards - construct patterns */
        let mut wildcard_patterns = [[0u8; 64]; 3];
        snprintf_concat(&mut wildcard_patterns[0], 64, &[b"*\0", pattern, b"*\0"]);
        snprintf_concat(&mut wildcard_patterns[1], 64, &[pattern, b"*\0"]);
        snprintf_concat(&mut wildcard_patterns[2], 64, &[b"*\0", pattern]);

        /* VULNERABLE: strcmp on unbounded text */
        for i in 0..3usize {
            if strcmp(text, &wildcard_patterns[i]) == 0 {
                return 2 + i as i32;
            }
        }

        /* Check if text contains pattern - uses strncmp safely */
        let text_len = strlen(text);
        let pattern_len = strlen(pattern);

        /* NOTE: `text_len - pattern_len` is computed with size_t arithmetic in
         * the C source.  When the pattern is longer than the text this wraps
         * around to a huge bound and the loop reads past the end of the
         * buffer - the original program dies from SIGSEGV, which the modelled
         * memory reproduces once the reads leave the buffer. */
        let bound = text_len.wrapping_sub(pattern_len);
        let mut i: usize = 0;
        while i <= bound {
            if strncmp(slice_from(text, i), pattern, pattern_len) == 0 {
                return (10usize.wrapping_add(i)) as i32; /* Return position + offset */
            }
            i += 1;
        }
    } else {
        /* Case-insensitive - need to check both cases */
        /* First try exact match - VULNERABLE */
        if strcmp(text, pattern) == 0 {
            return 1;
        }

        /* Manual case-insensitive comparison */
        let pattern_len = strlen(pattern);
        let text_len = strlen(text);

        if text_len != pattern_len {
            /* Try prefix match with strncmp */
            if strncmp(text, pattern, pattern_len) == 0 {
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
