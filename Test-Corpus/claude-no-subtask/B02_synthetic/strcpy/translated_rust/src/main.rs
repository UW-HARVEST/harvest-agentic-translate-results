// Translation of c_src/src/main.c + c_src/src/lib.c into safe Rust.
// Reproduces C's scanf/printf behavior and string-handling semantics
// (including bug-for-bug reads up to a NUL terminator within fixed-size
// buffers) to produce byte-identical output for valid inputs.

use std::io::{self, Read, Write};
use std::process::ExitCode;

const MAX_BUFFER_SIZE: usize = 1024;

// ---- stdin scanner that mimics scanf("%d"/"%u"/"%zu") whitespace handling ----

struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new() -> Self {
        let mut buf = Vec::new();
        // Best-effort read of all of stdin; if it fails, treat as EOF.
        let _ = io::stdin().read_to_end(&mut buf);
        Scanner { buf, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn read_token(&mut self) -> Option<&[u8]> {
        self.skip_ws();
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.buf.len() {
            let c = self.buf[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                break;
            }
            self.pos += 1;
        }
        Some(&self.buf[start..self.pos])
    }

    fn read_i32(&mut self) -> Option<i32> {
        let tok = self.read_token()?;
        std::str::from_utf8(tok).ok()?.parse::<i32>().ok()
    }

    fn read_u32(&mut self) -> Option<u32> {
        let tok = self.read_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        // %u accepts an optional sign in C; strip leading '+' if present.
        let s = s.strip_prefix('+').unwrap_or(s);
        s.parse::<u32>().ok()
    }

    fn read_usize(&mut self) -> Option<usize> {
        let tok = self.read_token()?;
        let s = std::str::from_utf8(tok).ok()?;
        let s = s.strip_prefix('+').unwrap_or(s);
        s.parse::<usize>().ok()
    }
}

// ---- Minimal C-string helpers operating on &[u8] / &mut [u8] ----

fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn c_strcmp(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let av = if i < a.len() { a[i] } else { 0 };
        let bv = if i < b.len() { b[i] } else { 0 };
        if av != bv {
            return (av as i32) - (bv as i32);
        }
        if av == 0 {
            return 0;
        }
        i += 1;
    }
}

fn c_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    for i in 0..n {
        let av = if i < a.len() { a[i] } else { 0 };
        let bv = if i < b.len() { b[i] } else { 0 };
        if av != bv {
            return (av as i32) - (bv as i32);
        }
        if av == 0 {
            return 0;
        }
    }
    0
}

fn c_strncpy(dst: &mut [u8], src: &[u8], n: usize) {
    let mut i = 0;
    let mut hit_null = false;
    while i < n && i < dst.len() {
        let c = if !hit_null && i < src.len() { src[i] } else { 0 };
        if c == 0 {
            hit_null = true;
        }
        dst[i] = c;
        i += 1;
    }
}

fn c_strncat(dst: &mut [u8], src: &[u8], n: usize) {
    let dst_end = c_strlen(dst);
    let mut written = 0;
    while written < n && dst_end + written < dst.len() {
        let c = if written < src.len() { src[written] } else { 0 };
        if c == 0 {
            break;
        }
        dst[dst_end + written] = c;
        written += 1;
    }
    if dst_end + written < dst.len() {
        dst[dst_end + written] = 0;
    }
}

// snprintf for the "*%s*" / "%s*" / "*%s" patterns used in lib.c.
// Writes prefix + (NUL-terminated portion of) middle + suffix into dst,
// truncating to dst.len()-1 bytes and always NUL-terminating.
fn snprintf_wrap(dst: &mut [u8], prefix: &[u8], middle: &[u8], suffix: &[u8]) {
    let cap = dst.len();
    if cap == 0 {
        return;
    }
    let max = cap - 1;
    let mut written = 0usize;

    for &c in prefix {
        if written >= max {
            break;
        }
        dst[written] = c;
        written += 1;
    }
    let mid_len = c_strlen(middle);
    let mut i = 0;
    while i < mid_len && written < max {
        dst[written] = middle[i];
        i += 1;
        written += 1;
    }
    for &c in suffix {
        if written >= max {
            break;
        }
        dst[written] = c;
        written += 1;
    }
    dst[written] = 0;
    // Anything past `written` is left as-is in dst (matches C snprintf semantics).
}

// ---- Library functions ported from lib.c ----

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    if c_strcmp(token, expected) == 0 {
        return 1;
    }
    if c_strcmp(token, b"VALID") == 0 || c_strcmp(token, b"OK") == 0 {
        return 1;
    }
    0
}

fn parse_command(buffer: &[u8], buf_size: usize, cmd_list: &[&[u8]]) -> i32 {
    for (i, cmd) in cmd_list.iter().enumerate() {
        let cmd_len = c_strlen(cmd);

        if buf_size >= cmd_len {
            if c_strncmp(buffer, cmd, cmd_len) == 0 {
                let next = if cmd_len < buffer.len() {
                    buffer[cmd_len]
                } else {
                    0
                };
                if next == 0 || next == b' ' {
                    return i as i32;
                }
            }
        }

        if c_strcmp(buffer, cmd) == 0 {
            return i as i32;
        }
    }

    if c_strcmp(buffer, b"ADMIN") == 0 {
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

        // char variations[5][32] = {"_v1", "_v2", "_old", "_new", "_tmp"};
        let variations: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];

        for (i, var) in variations.iter().enumerate() {
            // char expected[64];
            let mut expected = [0u8; 64];
            // strncpy(expected, prefix, 63);
            c_strncpy(&mut expected[..63], prefix, 63);
            // expected[63] = '\0';
            expected[63] = 0;
            // strncat(expected, variations[i], 63 - strlen(expected));
            let cur_len = c_strlen(&expected);
            let remaining = 63usize.saturating_sub(cur_len);
            c_strncat(&mut expected, var, remaining);

            if c_strcmp(s, &expected) == 0 {
                return (2 + i) as i32;
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

    for i in 0..len {
        let c = if i < data.len() { data[i] } else { 0 };
        if c == delim {
            return i as i32;
        }
        if c == 0 {
            break;
        }
    }

    if delim == b'|' && c_strcmp(data, b"NONE") == 0 {
        return -2;
    }

    if delim == b':' && c_strcmp(data, b"EMPTY") == 0 {
        return -3;
    }

    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    if case_sensitive {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        // char wildcard_patterns[3][64];
        let mut wildcard_patterns: [[u8; 64]; 3] = [[0; 64]; 3];
        // snprintf(wildcard_patterns[0], 64, "*%s*", pattern);
        snprintf_wrap(&mut wildcard_patterns[0], b"*", pattern, b"*");
        // snprintf(wildcard_patterns[1], 64, "%s*", pattern);
        snprintf_wrap(&mut wildcard_patterns[1], b"", pattern, b"*");
        // snprintf(wildcard_patterns[2], 64, "*%s", pattern);
        snprintf_wrap(&mut wildcard_patterns[2], b"*", pattern, b"");

        for i in 0..3 {
            if c_strcmp(text, &wildcard_patterns[i]) == 0 {
                return (2 + i) as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        // for (size_t i = 0; i <= text_len - pattern_len; i++)
        // If pattern_len > text_len, the C version underflows (UB);
        // we treat it as no match found, matching the well-defined cases.
        if pattern_len <= text_len {
            let upper = text_len - pattern_len;
            for i in 0..=upper {
                if i >= text.len() {
                    break;
                }
                if c_strncmp(&text[i..], pattern, pattern_len) == 0 {
                    return (10 + i) as i32;
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
            let mut matched = true;
            for i in 0..pattern_len {
                let mut c1 = if i < text.len() { text[i] } else { 0 };
                let mut c2 = if i < pattern.len() { pattern[i] } else { 0 };
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

    0
}

fn process_strings(
    input: &[u8],
    input_len: usize,
    reference: &[u8],
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    // The C code checks `input == NULL`; in main, input_buffer is always
    // a valid stack array, so this branch is never taken. Reference is
    // likewise always non-NULL when passed from main, so the NULL checks
    // below are kept for fidelity but never trigger here.

    match operation {
        0 => validate_token(input, reference),
        1 => {
            let commands: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];
            parse_command(input, input_len, &commands)
        }
        2 => {
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = if ref_len > 0 && !reference.is_empty() {
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

// ---- main ----

fn run() -> i32 {
    let mut sc = Scanner::new();

    // Read operation
    let operation = match sc.read_i32() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return 1;
        }
    };

    // Read flags
    let flags = match sc.read_u32() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading flags");
            return 1;
        }
    };

    // Read input length
    let input_len = match sc.read_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading input length");
            return 1;
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            io::stderr(),
            "Error: input length {} exceeds maximum {}",
            input_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Buffers are stack-allocated and uninitialized in C; here we
    // zero-initialize so reads past `input_len` see deterministic bytes.
    let mut input_buffer = [0u8; MAX_BUFFER_SIZE];
    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE];

    // Read input bytes
    for i in 0..input_len {
        let byte = match sc.read_u32() {
            Some(v) => v,
            None => {
                let _ = writeln!(io::stderr(), "Error reading input byte {}", i);
                return 1;
            }
        };
        input_buffer[i] = byte as u8; // (char)byte truncates to low 8 bits
    }

    // Read reference length
    let ref_len = match sc.read_usize() {
        Some(v) => v,
        None => {
            let _ = writeln!(io::stderr(), "Error reading reference length");
            return 1;
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        let _ = writeln!(
            io::stderr(),
            "Error: reference length {} exceeds maximum {}",
            ref_len, MAX_BUFFER_SIZE
        );
        return 1;
    }

    // Read reference bytes
    for i in 0..ref_len {
        let byte = match sc.read_u32() {
            Some(v) => v,
            None => {
                let _ = writeln!(io::stderr(), "Error reading reference byte {}", i);
                return 1;
            }
        };
        ref_buffer[i] = byte as u8;
    }

    let result = process_strings(
        &input_buffer,
        input_len,
        &ref_buffer,
        ref_len,
        operation,
        flags,
    );

    println!("{}", result);
    0
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
