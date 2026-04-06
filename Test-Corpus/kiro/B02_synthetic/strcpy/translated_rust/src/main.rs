use std::io::{self, Read};

const MAX_BUFFER_SIZE: usize = 1024;

// ---- C-string helpers operating on byte slices ----

/// Equivalent to C strlen: find first 0 byte
fn c_strlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// Equivalent to C strcmp: compare bytes until a NUL or difference
fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca == 0 && cb == 0 {
            return 0;
        }
        if ca != cb {
            return (ca as i32) - (cb as i32);
        }
        i += 1;
    }
}

/// Equivalent to C strncmp
fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
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

/// Equivalent to C strncpy(dst, src, n) — copies up to n bytes, pads with 0
fn c_strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let mut found_nul = false;
    for i in 0..n {
        if !found_nul {
            let c = if i < src.len() { src[i] } else { 0 };
            if c == 0 {
                found_nul = true;
            }
            dst[i] = c;
        } else {
            dst[i] = 0;
        }
    }
}

/// Equivalent to C strncat(dst, src, n)
fn c_strncat(dst: &mut [u8], src: &[u8], n: usize) {
    let dst_len = c_strlen(dst);
    let mut j = 0;
    while j < n {
        let c = if j < src.len() { src[j] } else { 0 };
        if c == 0 {
            break;
        }
        if dst_len + j < dst.len() {
            dst[dst_len + j] = c;
        }
        j += 1;
    }
    if dst_len + j < dst.len() {
        dst[dst_len + j] = 0;
    }
}

/// Equivalent to C snprintf(buf, size, "*%s*", pattern) etc.
fn snprintf_wrap(buf: &mut [u8], fmt: &str, pattern: &[u8]) {
    let pat_str_len = c_strlen(pattern);
    let pat = &pattern[..pat_str_len];
    let max = buf.len().saturating_sub(1);
    let mut pos = 0;
    for &c in fmt.as_bytes() {
        if c == b'%' {
            // skip the '%s' and insert pattern
            continue;
        }
        if c == b's' && pos == 0 || (pos > 0 && buf[pos - 1] == 0) {
            // already handled
            continue;
        }
        // This is a simplified approach; let's do it properly below
        break;
    }
    // Proper implementation: scan fmt for %s and substitute
    pos = 0;
    let fmt_bytes = fmt.as_bytes();
    let mut fi = 0;
    while fi < fmt_bytes.len() && pos < max {
        if fi + 1 < fmt_bytes.len() && fmt_bytes[fi] == b'%' && fmt_bytes[fi + 1] == b's' {
            // insert pattern
            for &b in pat {
                if pos >= max {
                    break;
                }
                buf[pos] = b;
                pos += 1;
            }
            fi += 2;
        } else {
            buf[pos] = fmt_bytes[fi];
            pos += 1;
            fi += 1;
        }
    }
    buf[pos] = 0;
}

// ---- Translated library functions ----

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1;
    }
    if c_strcmp(token, b"VALID\0") == 0 || c_strcmp(token, b"OK\0") == 0 {
        return 1;
    }
    0
}

fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]], list_size: i32) -> i32 {
    for i in 0..list_size as usize {
        let cmd_len = c_strlen(cmd_list[i]);
        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd_list[i], cmd_len) == 0 {
                let c = if cmd_len < buffer.len() { buffer[cmd_len] } else { 0 };
                if c == 0 || c == b' ' {
                    return i as i32;
                }
            }
        }
        if c_strcmp(buffer, cmd_list[i]) == 0 {
            return i as i32;
        }
    }
    if c_strcmp(buffer, b"ADMIN\0") == 0 {
        return 99;
    }
    -1
}

fn compare_prefix(str_buf: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match != 0 {
        if c_strcmp(str_buf, prefix) == 0 {
            return 1;
        }
        let variations: [&[u8]; 5] = [b"_v1\0", b"_v2\0", b"_old\0", b"_new\0", b"_tmp\0"];
        for i in 0..5 {
            let mut expected = [0u8; 64];
            c_strncpy(&mut expected, prefix, 63);
            expected[63] = 0;
            let elen = c_strlen(&expected);
            c_strncat(&mut expected, variations[i], 63usize.saturating_sub(elen));
            if c_strcmp(str_buf, &expected) == 0 {
                return 2 + i as i32;
            }
        }
        0
    } else {
        if c_strncmp(str_buf, prefix, prefix_len) == 0 {
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
    if delim == b'|' && c_strcmp(data, b"NONE\0") == 0 {
        return -2;
    }
    if delim == b':' && c_strcmp(data, b"EMPTY\0") == 0 {
        return -3;
    }
    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }
        let mut wildcard_patterns = [[0u8; 64]; 3];
        snprintf_wrap(&mut wildcard_patterns[0], "*%s*", pattern);
        snprintf_wrap(&mut wildcard_patterns[1], "%s*", pattern);
        snprintf_wrap(&mut wildcard_patterns[2], "*%s", pattern);

        for i in 0..3 {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return 2 + i as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        // C code: for (size_t i = 0; i <= text_len - pattern_len; ...)
        // When pattern_len > text_len, C wraps around (unsigned). Reproduce:
        // In that case the loop condition is huge, but data[i] will likely
        // not match, and we'd go out of bounds. In practice the C would
        // read garbage or crash. We replicate the wrapping behavior:
        if pattern_len <= text_len {
            let limit = text_len - pattern_len;
            for i in 0..=limit {
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
                let mut c1 = text[i];
                let mut c2 = pattern[i];
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

fn process_strings_impl(
    input: &mut [u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // input == NULL check: can't happen in safe Rust, skip (C caller always passes valid buffer)

    match operation {
        0 => {
            // reference == NULL can't happen here
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
            parse_command(input, input_len, &commands, 5)
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

// ---- C-compatible exported entry point ----

#[no_mangle]
pub unsafe extern "C" fn process_strings(
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
    let inp = std::slice::from_raw_parts_mut(input, MAX_BUFFER_SIZE);
    let refer = if reference.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(reference, MAX_BUFFER_SIZE)
    };
    process_strings_impl(inp, input_len, refer, ref_len, operation, flags)
}

// ---- scanf-style token reader ----

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data).unwrap_or(0);
        Scanner { data, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn next_token(&mut self) -> Option<&[u8]> {
        self.skip_whitespace();
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.data.len() && !self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        Some(&self.data[start..self.pos])
    }

    fn scan_int(&mut self) -> Option<i32> {
        let tok = self.next_token()?;
        std::str::from_utf8(tok).ok()?.parse().ok()
    }

    fn scan_u32(&mut self) -> Option<u32> {
        let tok = self.next_token()?;
        std::str::from_utf8(tok).ok()?.parse().ok()
    }

    fn scan_usize(&mut self) -> Option<usize> {
        let tok = self.next_token()?;
        std::str::from_utf8(tok).ok()?.parse().ok()
    }
}

fn main() {
    let mut sc = Scanner::new();

    let operation = match sc.scan_int() {
        Some(v) => v,
        None => {
            eprintln!("Error reading operation");
            std::process::exit(1);
        }
    };

    let flags = match sc.scan_u32() {
        Some(v) => v,
        None => {
            eprintln!("Error reading flags");
            std::process::exit(1);
        }
    };

    let input_len = match sc.scan_usize() {
        Some(v) => v,
        None => {
            eprintln!("Error reading input length");
            std::process::exit(1);
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        eprintln!(
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut input_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..input_len {
        match sc.scan_u32() {
            Some(byte) => input_buffer[i] = byte as u8,
            None => {
                eprintln!("Error reading input byte {}", i);
                std::process::exit(1);
            }
        }
    }

    let ref_len = match sc.scan_usize() {
        Some(v) => v,
        None => {
            eprintln!("Error reading reference length");
            std::process::exit(1);
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        eprintln!(
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        std::process::exit(1);
    }

    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE];
    for i in 0..ref_len {
        match sc.scan_u32() {
            Some(byte) => ref_buffer[i] = byte as u8,
            None => {
                eprintln!("Error reading reference byte {}", i);
                std::process::exit(1);
            }
        }
    }

    let result = process_strings_impl(
        &mut input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
}
