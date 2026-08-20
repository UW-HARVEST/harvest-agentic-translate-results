//! Translation of `c_src/src/lib.c`.
//!
//! `char *` parameters are byte offsets into the emulated stack frame
//! (`crate::mem`); `NULL_OFF` stands in for a NULL pointer.  All string
//! operations keep the original (unbounded, potentially out-of-bounds)
//! behaviour of the C code.

use crate::cstr::{
    as_cstr_local, snprintf_concat, strcmp_mem_bytes, strcmp_mem_mem, strlen_local, strlen_mem,
    strncat_local, strncmp_mem_bytes, strncmp_mem_mem, strncpy_from_mem,
};
use crate::mem::{Mem, NULL_OFF};

/// Main entrance function - performs various string comparison operations
pub fn process_strings(
    mem: &Mem,
    input: usize,
    input_len: usize,
    reference: usize,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    if input == NULL_OFF {
        return -1;
    }

    /* Different operations based on operation code */
    match operation {
        0 => {
            /* Validate token - VULNERABLE if input not null-terminated */
            if reference == NULL_OFF {
                return -2;
            }
            validate_token(mem, input, reference)
        }

        1 => {
            /* Parse command from list - checks against multiple strings */
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(mem, input, input_len, &commands, 5)
        }

        2 => {
            /* Compare prefix - can use strcmp or strncmp based on flags */
            if reference == NULL_OFF {
                return -2;
            }
            let exact = (flags & 0x01) as i32;
            compare_prefix(mem, input, reference, exact)
        }

        3 => {
            /* Find delimiter position */
            let delim = if reference != NULL_OFF && ref_len > 0 {
                mem.get(reference)
            } else {
                b':'
            };
            find_delimiter(mem, input, input_len, delim)
        }

        4 => {
            /* Match pattern - VULNERABLE in certain paths */
            if reference == NULL_OFF {
                return -2;
            }
            let case_sens = (flags & 0x02) as i32;
            match_pattern(mem, input, reference, case_sens)
        }

        _ => -3,
    }
}

/// Validate token against expected value
/// VULNERABLE: Uses strcmp without ensuring null termination
fn validate_token(mem: &Mem, token: usize, expected: usize) -> i32 {
    /* Direct strcmp - will overflow if token not null-terminated */
    if strcmp_mem_mem(mem, token, expected) == 0 {
        return 1; /* Valid */
    }

    /* Also check some common variations */
    if strcmp_mem_bytes(mem, token, b"VALID") == 0 || strcmp_mem_bytes(mem, token, b"OK") == 0 {
        return 1;
    }

    0 /* Invalid */
}

/// Parse command from a list of valid commands
/// Mix of safe and unsafe comparisons
fn parse_command(
    mem: &Mem,
    buffer: usize,
    buf_size: usize,
    cmd_list: &[&[u8]],
    list_size: i32,
) -> i32 {
    /* Iterate through command list */
    for i in 0..list_size {
        let cmd = cmd_list[i as usize];
        /* Safe comparison using strncmp first */
        let cmd_len = cmd.len();

        if buf_size >= cmd_len {
            if strncmp_mem_bytes(mem, buffer, cmd, cmd_len) == 0 {
                /* Check if exact match - VULNERABLE if buffer not null-terminated */
                let c = mem.get(buffer + cmd_len);
                if c == 0 || c == b' ' {
                    return i; /* Return command index */
                }
            }
        }

        /* Fallback: direct strcmp - VULNERABLE */
        if strcmp_mem_bytes(mem, buffer, cmd) == 0 {
            return i;
        }
    }

    /* Check for special admin command - always vulnerable strcmp */
    if strcmp_mem_bytes(mem, buffer, b"ADMIN") == 0 {
        return 99;
    }

    -1 /* No match */
}

/// Compare prefix with optional exact matching
/// Safe when exact_match=0, vulnerable when exact_match=1
fn compare_prefix(mem: &Mem, str_: usize, prefix: usize, exact_match: i32) -> i32 {
    let prefix_len = strlen_mem(mem, prefix);

    if exact_match != 0 {
        /* Exact match required - VULNERABLE strcmp */
        if strcmp_mem_mem(mem, str_, prefix) == 0 {
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
            strncpy_from_mem(&mut expected, mem, prefix, 63);
            expected[63] = 0;
            let n = 63 - strlen_local(&expected);
            let variation = variations[i];
            strncat_local(&mut expected, as_cstr_local(&variation), n);

            /* VULNERABLE: strcmp without length check on str */
            if strcmp_mem_bytes(mem, str_, as_cstr_local(&expected)) == 0 {
                return 2 + i as i32;
            }
        }

        0
    } else {
        /* Prefix match only - safer with strncmp */
        if strncmp_mem_mem(mem, str_, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

/// Find delimiter position in string
/// Uses strncmp for safety but has edge cases
fn find_delimiter(mem: &Mem, data: usize, len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    /* Manual search with bounds checking */
    for i in 0..len {
        let c = mem.get(data + i);
        if c == delim {
            return i as i32;
        }
        if c == 0 {
            break;
        }
    }

    /* Check for special delimiter patterns using strcmp */
    /* VULNERABLE: doesn't verify data is null-terminated */
    if delim == b'|' && strcmp_mem_bytes(mem, data, b"NONE") == 0 {
        return -2; /* Special case */
    }

    if delim == b':' && strcmp_mem_bytes(mem, data, b"EMPTY") == 0 {
        return -3; /* Special case */
    }

    -1 /* Not found */
}

/// Match pattern with optional case sensitivity
/// Multiple vulnerable strcmp calls
fn match_pattern(mem: &Mem, text: usize, pattern: usize, case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        /* Case-sensitive exact match - VULNERABLE */
        if strcmp_mem_mem(mem, text, pattern) == 0 {
            return 1;
        }

        /* Try with wildcards - construct patterns */
        let pattern_chars = read_cstr_mem(mem, pattern);
        let mut wildcard_patterns = [[0u8; 64]; 3];
        snprintf_concat(&mut wildcard_patterns[0], &[b"*", &pattern_chars, b"*"]);
        snprintf_concat(&mut wildcard_patterns[1], &[&pattern_chars, b"*"]);
        snprintf_concat(&mut wildcard_patterns[2], &[b"*", &pattern_chars]);

        /* VULNERABLE: strcmp on unbounded text */
        for i in 0..3usize {
            let wildcard = wildcard_patterns[i];
            if strcmp_mem_bytes(mem, text, as_cstr_local(&wildcard)) == 0 {
                return 2 + i as i32;
            }
        }

        /* Check if text contains pattern - uses strncmp safely */
        let text_len = strlen_mem(mem, text);
        let pattern_len = strlen_mem(mem, pattern);

        /* The bound underflows when the pattern is longer than the text, just
         * like the unsigned arithmetic in the C code. */
        let bound = text_len.wrapping_sub(pattern_len);
        let mut i = 0usize;
        while i <= bound {
            if strncmp_mem_mem(mem, text + i, pattern, pattern_len) == 0 {
                return (10 + i) as i32; /* Return position + offset */
            }
            i += 1;
        }
    } else {
        /* Case-insensitive - need to check both cases */
        /* First try exact match - VULNERABLE */
        if strcmp_mem_mem(mem, text, pattern) == 0 {
            return 1;
        }

        /* Manual case-insensitive comparison */
        let pattern_len = strlen_mem(mem, pattern);
        let text_len = strlen_mem(mem, text);

        if text_len != pattern_len {
            /* Try prefix match with strncmp */
            if strncmp_mem_mem(mem, text, pattern, pattern_len) == 0 {
                return 5;
            }
        }

        /* Compare character by character (safer) */
        if text_len == pattern_len {
            let mut matched = 1;
            for i in 0..pattern_len {
                let mut c1 = mem.get(text + i);
                let mut c2 = mem.get(pattern + i);

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

/// Characters of the NUL terminated string at `off` (used to feed `snprintf`).
fn read_cstr_mem(mem: &Mem, off: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0usize;
    loop {
        let c = mem.get(off + i);
        if c == 0 {
            return out;
        }
        out.push(c);
        i += 1;
    }
}
