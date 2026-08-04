use std::io::{self, Read};

const MAX_BUFFER_SIZE: usize = 1024;

/// C-style strlen: find first null byte
fn cstrlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// C-style strcmp: compare two null-terminated byte slices
fn strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
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

/// C-style strncmp
fn strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
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

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if strcmp(token, expected) == 0 {
        return 1;
    }
    if strcmp(token, b"VALID\0") == 0 || strcmp(token, b"OK\0") == 0 {
        return 1;
    }
    0
}

fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = cstrlen(cmd);
        if buf_size >= cmd_len {
            if strncmp(buffer, cmd, cmd_len) == 0 {
                let b = if cmd_len < buffer.len() { buffer[cmd_len] } else { 0 };
                if b == 0 || b == b' ' {
                    return i as i32;
                }
            }
        }
        if strcmp(buffer, cmd) == 0 {
            return i as i32;
        }
    }
    if strcmp(buffer, b"ADMIN\0") == 0 {
        return 99;
    }
    -1
}

fn compare_prefix(str_buf: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = cstrlen(prefix);

    if exact_match != 0 {
        if strcmp(str_buf, prefix) == 0 {
            return 1;
        }
        let variations: [&[u8]; 5] = [b"_v1\0", b"_v2\0", b"_old\0", b"_new\0", b"_tmp\0"];
        for (i, var) in variations.iter().enumerate() {
            // strncpy(expected, prefix, 63); expected[63]='\0'; strncat(expected, var, 63-strlen(expected))
            let mut expected = [0u8; 64];
            let copy_len = prefix_len.min(63);
            expected[..copy_len].copy_from_slice(&prefix[..copy_len]);
            expected[63] = 0;
            let elen = cstrlen(&expected);
            let var_len = cstrlen(var);
            let cat_max = 63usize.saturating_sub(elen);
            let cat_len = var_len.min(cat_max);
            expected[elen..elen + cat_len].copy_from_slice(&var[..cat_len]);
            // null terminate after cat
            if elen + cat_len < 64 {
                expected[elen + cat_len] = 0;
            }
            if strcmp(str_buf, &expected) == 0 {
                return 2 + i as i32;
            }
        }
        0
    } else {
        if strncmp(str_buf, prefix, prefix_len) == 0 {
            return 1;
        }
        0
    }
}

fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }
    for i in 0..len {
        if i < data.len() {
            if data[i] == delim {
                return i as i32;
            }
            if data[i] == 0 {
                break;
            }
        }
    }
    if delim == b'|' && strcmp(data, b"NONE\0") == 0 {
        return -2;
    }
    if delim == b':' && strcmp(data, b"EMPTY\0") == 0 {
        return -3;
    }
    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        if strcmp(text, pattern) == 0 {
            return 1;
        }
        // Build wildcard patterns using snprintf-like logic
        let pat_str_len = cstrlen(pattern);
        let mut wp = [[0u8; 64]; 3];
        // "*%s*"
        {
            let w = &mut wp[0];
            w[0] = b'*';
            let copy = pat_str_len.min(62);
            w[1..1 + copy].copy_from_slice(&pattern[..copy]);
            let pos = 1 + copy;
            if pos < 64 {
                w[pos] = b'*';
                if pos + 1 < 64 {
                    w[pos + 1] = 0;
                }
            }
        }
        // "%s*"
        {
            let w = &mut wp[1];
            let copy = pat_str_len.min(63);
            w[..copy].copy_from_slice(&pattern[..copy]);
            if copy < 64 {
                w[copy] = b'*';
                if copy + 1 < 64 {
                    w[copy + 1] = 0;
                }
            }
        }
        // "*%s"
        {
            let w = &mut wp[2];
            w[0] = b'*';
            let copy = pat_str_len.min(63);
            w[1..1 + copy].copy_from_slice(&pattern[..copy]);
            if 1 + copy < 64 {
                w[1 + copy] = 0;
            }
        }
        for i in 0..3 {
            if strcmp(text, &wp[i]) == 0 {
                return 2 + i as i32;
            }
        }
        // Substring search
        let text_len = cstrlen(text);
        let pattern_len = cstrlen(pattern);
        // C code: for (size_t i = 0; i <= text_len - pattern_len; i++)
        // When text_len < pattern_len, text_len - pattern_len wraps to huge value in C (size_t),
        // but the strncmp calls would just fail. We replicate by skipping if text_len < pattern_len.
        if text_len >= pattern_len {
            for i in 0..=(text_len - pattern_len) {
                if strncmp(&text[i..], pattern, pattern_len) == 0 {
                    return 10 + i as i32;
                }
            }
        }
    } else {
        if strcmp(text, pattern) == 0 {
            return 1;
        }
        let pattern_len = cstrlen(pattern);
        let text_len = cstrlen(text);
        if text_len != pattern_len {
            if strncmp(text, pattern, pattern_len) == 0 {
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

fn process_strings(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // input == NULL check: in Rust we always have a valid slice, but the C code
    // would only hit NULL if no buffer was passed. We skip this since main always passes a buffer.

    match operation {
        0 => {
            // reference == NULL not possible from our main
            validate_token(input, reference)
        }
        1 => {
            let commands: &[&[u8]] = &[
                b"START\0", b"STOP\0", b"PAUSE\0", b"RESUME\0", b"RESET\0",
            ];
            parse_command(input, input_len, commands)
        }
        2 => {
            let exact = (flags & 0x01) as i32;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = if ref_len > 0 { reference[0] } else { b':' };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            let case_sens = (flags & 0x02) as i32;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

#[no_mangle]
pub unsafe extern "C" fn process_strings_ffi(
    input: *mut u8,
    input_len: usize,
    reference: *const u8,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    if input.is_null() {
        return -1;
    }
    let inp = unsafe { std::slice::from_raw_parts_mut(input, 1024) };
    let refr = if reference.is_null() {
        match operation {
            0 | 2 | 4 => return -2,
            3 => &[] as &[u8],
            _ => &[] as &[u8],
        }
    } else {
        unsafe { std::slice::from_raw_parts(reference, 1024) }
    };
    process_strings(inp, input_len, refr, ref_len, operation, flags)
}

fn main() {
    let mut all_input = String::new();
    io::stdin().read_to_string(&mut all_input).unwrap_or(0);
    let mut tokens = all_input.split_whitespace();

    macro_rules! next_token {
        ($err:expr) => {
            match tokens.next() {
                Some(t) => t,
                None => {
                    eprint!("{}\n", $err);
                    std::process::exit(1);
                }
            }
        };
    }

    macro_rules! parse_token {
        ($t:expr, $ty:ty, $err:expr) => {
            match $t.parse::<$ty>() {
                Ok(v) => v,
                Err(_) => {
                    eprint!("{}\n", $err);
                    std::process::exit(1);
                }
            }
        };
    }

    let tok = next_token!("Error reading operation");
    let operation: i32 = parse_token!(tok, i32, "Error reading operation");

    let tok = next_token!("Error reading flags");
    let flags: u32 = parse_token!(tok, u32, "Error reading flags");

    let tok = next_token!("Error reading input length");
    let input_len: usize = parse_token!(tok, usize, "Error reading input length");

    if input_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut input_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..input_len {
        let tok = next_token!(&format!("Error reading input byte {}", i));
        let byte: u32 = parse_token!(tok, u32, &format!("Error reading input byte {}", i));
        input_buffer[i] = byte as u8;
    }

    let tok = next_token!("Error reading reference length");
    let ref_len: usize = parse_token!(tok, usize, "Error reading reference length");

    if ref_len > MAX_BUFFER_SIZE {
        eprint!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..ref_len {
        let tok = next_token!(&format!("Error reading reference byte {}", i));
        let byte: u32 = parse_token!(tok, u32, &format!("Error reading reference byte {}", i));
        ref_buffer[i] = byte as u8;
    }

    let result = process_strings(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
}
