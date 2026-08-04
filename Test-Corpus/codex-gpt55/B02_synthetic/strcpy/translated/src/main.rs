use std::io::{self, Read, Write};

const MAX_BUFFER_SIZE: usize = 1024;

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn scan_number(&mut self) -> Option<(bool, u128)> {
        self.skip_ws();
        if self.pos >= self.data.len() {
            return None;
        }

        let mut negative = false;
        if self.data[self.pos] == b'+' || self.data[self.pos] == b'-' {
            negative = self.data[self.pos] == b'-';
            self.pos += 1;
        }

        let start = self.pos;
        let mut value = 0u128;
        while self.pos < self.data.len() && self.data[self.pos].is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add((self.data[self.pos] - b'0') as u128);
            self.pos += 1;
        }

        if self.pos == start {
            None
        } else {
            Some((negative, value))
        }
    }

    fn scan_i32(&mut self) -> Option<i32> {
        let (negative, value) = self.scan_number()?;
        if negative {
            Some((0i128.wrapping_sub(value as i128)) as i32)
        } else {
            Some(value as i32)
        }
    }

    fn scan_u32(&mut self) -> Option<u32> {
        let (negative, value) = self.scan_number()?;
        if negative {
            Some((0u32).wrapping_sub(value as u32))
        } else {
            Some(value as u32)
        }
    }

    fn scan_usize(&mut self) -> Option<usize> {
        let (negative, value) = self.scan_number()?;
        if negative {
            Some((0usize).wrapping_sub(value as usize))
        } else {
            Some(value as usize)
        }
    }
}

fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn c_strcmp(left: &[u8], right: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let a = left.get(i).copied().unwrap_or(0);
        let b = right.get(i).copied().unwrap_or(0);
        if a != b {
            return a as i32 - b as i32;
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
}

fn c_strncmp(left: &[u8], right: &[u8], n: usize) -> i32 {
    for i in 0..n {
        let a = left.get(i).copied().unwrap_or(0);
        let b = right.get(i).copied().unwrap_or(0);
        if a != b {
            return a as i32 - b as i32;
        }
        if a == 0 {
            return 0;
        }
    }
    0
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

        if buf_size >= cmd_len && c_strncmp(buffer, cmd, cmd_len) == 0 {
            let next = buffer.get(cmd_len).copied().unwrap_or(0);
            if next == 0 || next == b' ' {
                return i as i32;
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

fn compare_prefix(str_buf: &[u8], prefix: &[u8], exact_match: i32) -> i32 {
    let prefix_len = c_strlen(prefix);

    if exact_match != 0 {
        if c_strcmp(str_buf, prefix) == 0 {
            return 1;
        }

        let variations: [&[u8]; 5] = [b"_v1\0", b"_v2\0", b"_old\0", b"_new\0", b"_tmp\0"];
        for (i, variation) in variations.iter().enumerate() {
            let mut expected = [0u8; 64];
            let copy_len = prefix_len.min(63);
            expected[..copy_len].copy_from_slice(&prefix[..copy_len]);
            expected[63] = 0;

            let current_len = c_strlen(&expected);
            let max_append = 63usize.saturating_sub(current_len);
            let variation_len = c_strlen(variation);
            let append_len = variation_len.min(max_append);
            expected[current_len..current_len + append_len]
                .copy_from_slice(&variation[..append_len]);
            if current_len + append_len < expected.len() {
                expected[current_len + append_len] = 0;
            }

            if c_strcmp(str_buf, &expected) == 0 {
                return 2 + i as i32;
            }
        }

        0
    } else if c_strncmp(str_buf, prefix, prefix_len) == 0 {
        1
    } else {
        0
    }
}

fn find_delimiter(data: &[u8], len: usize, delim: u8) -> i32 {
    if len == 0 {
        return -1;
    }

    for i in 0..len {
        let b = data.get(i).copied().unwrap_or(0);
        if b == delim {
            return i as i32;
        }
        if b == 0 {
            break;
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

fn snprintf_wildcard(kind: usize, pattern: &[u8]) -> [u8; 64] {
    let mut out = [0u8; 64];
    let pat_len = c_strlen(pattern);
    let mut pos = 0usize;

    if kind == 0 || kind == 2 {
        out[pos] = b'*';
        pos += 1;
    }

    let room_for_pattern = match kind {
        0 => 61,
        1 | 2 => 62,
        _ => 0,
    };
    let copy_len = pat_len.min(room_for_pattern);
    out[pos..pos + copy_len].copy_from_slice(&pattern[..copy_len]);
    pos += copy_len;

    if kind == 0 || kind == 1 {
        if pos < 63 {
            out[pos] = b'*';
            pos += 1;
        }
    }

    if pos < 64 {
        out[pos] = 0;
    }
    out
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: i32) -> i32 {
    if case_sensitive != 0 {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        let wildcard_patterns = [
            snprintf_wildcard(0, pattern),
            snprintf_wildcard(1, pattern),
            snprintf_wildcard(2, pattern),
        ];

        for (i, wildcard) in wildcard_patterns.iter().enumerate() {
            if c_strcmp(text, wildcard) == 0 {
                return 2 + i as i32;
            }
        }

        let text_len = c_strlen(text);
        let pattern_len = c_strlen(pattern);

        let mut i = 0usize;
        while i <= text_len.wrapping_sub(pattern_len) {
            if c_strncmp(&text[i..], pattern, pattern_len) == 0 {
                return 10 + i as i32;
            }
            i += 1;
        }
    } else {
        if c_strcmp(text, pattern) == 0 {
            return 1;
        }

        let pattern_len = c_strlen(pattern);
        let text_len = c_strlen(text);

        if text_len != pattern_len && c_strncmp(text, pattern, pattern_len) == 0 {
            return 5;
        }

        if text_len == pattern_len {
            let mut is_match = 1;
            for i in 0..pattern_len {
                let mut c1 = text.get(i).copied().unwrap_or(0);
                let mut c2 = pattern.get(i).copied().unwrap_or(0);

                if c1.is_ascii_uppercase() {
                    c1 += 32;
                }
                if c2.is_ascii_uppercase() {
                    c2 += 32;
                }

                if c1 != c2 {
                    is_match = 0;
                    break;
                }
            }
            if is_match != 0 {
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
    match operation {
        0 => validate_token(input, reference),
        1 => {
            let commands: [&[u8]; 5] = [b"START\0", b"STOP\0", b"PAUSE\0", b"RESUME\0", b"RESET\0"];
            parse_command(input, input_len, &commands)
        }
        2 => compare_prefix(input, reference, (flags & 0x01) as i32),
        3 => {
            let delim = if ref_len > 0 { reference[0] } else { b':' };
            find_delimiter(input, input_len, delim)
        }
        4 => match_pattern(input, reference, (flags & 0x02) as i32),
        _ => -3,
    }
}

fn main() {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).unwrap();
    let mut scanner = Scanner::new(bytes);

    let operation = match scanner.scan_i32() {
        Some(value) => value,
        None => {
            eprintln!("Error reading operation");
            std::process::exit(1);
        }
    };

    let flags = match scanner.scan_u32() {
        Some(value) => value,
        None => {
            eprintln!("Error reading flags");
            std::process::exit(1);
        }
    };

    let input_len = match scanner.scan_usize() {
        Some(value) => value,
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

    let mut input_buffer = [0u8; MAX_BUFFER_SIZE + 1];
    for i in 0..input_len {
        let byte = match scanner.scan_u32() {
            Some(value) => value,
            None => {
                eprintln!("Error reading input byte {}", i);
                std::process::exit(1);
            }
        };
        input_buffer[i] = byte as u8;
    }

    let ref_len = match scanner.scan_usize() {
        Some(value) => value,
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

    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE + 1];
    for i in 0..ref_len {
        let byte = match scanner.scan_u32() {
            Some(value) => value,
            None => {
                eprintln!("Error reading reference byte {}", i);
                std::process::exit(1);
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

    let mut stdout = io::stdout();
    writeln!(stdout, "{}", result).unwrap();
}
