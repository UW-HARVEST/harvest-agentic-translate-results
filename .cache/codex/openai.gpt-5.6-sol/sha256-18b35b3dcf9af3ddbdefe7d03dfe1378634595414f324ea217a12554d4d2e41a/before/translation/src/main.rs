use std::io::{self, Read, Write};

const MAX_BUFFER_SIZE: usize = 1024;
const REFERENCE_OFFSET: usize = MAX_BUFFER_SIZE;
const MEMORY_SIZE: usize = MAX_BUFFER_SIZE * 2 + 1;

struct Scanner {
    data: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn from_stdin() -> Self {
        let mut data = Vec::new();
        let _ = io::stdin().read_to_end(&mut data);
        Self { data, position: 0 }
    }

    fn decimal_parts(&mut self) -> Option<(bool, &[u8])> {
        while self
            .data
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            self.position += 1;
        }

        let mut negative = false;
        if let Some(sign) = self.data.get(self.position) {
            if *sign == b'+' || *sign == b'-' {
                negative = *sign == b'-';
                self.position += 1;
            }
        }

        let start = self.position;
        while self.data.get(self.position).is_some_and(u8::is_ascii_digit) {
            self.position += 1;
        }

        (self.position != start).then(|| (negative, &self.data[start..self.position]))
    }

    fn scan_i32(&mut self) -> Option<i32> {
        let (negative, digits) = self.decimal_parts()?;
        let negative_limit = i64::MAX as u128 + 1;
        let limit = if negative {
            negative_limit
        } else {
            i64::MAX as u128
        };
        let (magnitude, overflow) = parse_magnitude(digits, limit);

        let value = if overflow {
            if negative {
                i64::MIN
            } else {
                i64::MAX
            }
        } else if negative {
            if magnitude == negative_limit {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }

    fn scan_usize(&mut self) -> Option<usize> {
        let (negative, digits) = self.decimal_parts()?;
        let (magnitude, overflow) = parse_magnitude(digits, usize::MAX as u128);
        if overflow {
            Some(usize::MAX)
        } else {
            let value = magnitude as usize;
            Some(if negative {
                value.wrapping_neg()
            } else {
                value
            })
        }
    }

    fn scan_u32(&mut self) -> Option<u32> {
        self.scan_usize().map(|value| value as u32)
    }
}

fn parse_magnitude(digits: &[u8], limit: u128) -> (u128, bool) {
    let mut value = 0_u128;
    let mut overflow = false;

    for &digit in digits {
        let digit = (digit - b'0') as u128;
        if value > (limit - digit) / 10 {
            overflow = true;
            value = limit;
        } else if !overflow {
            value = value * 10 + digit;
        }
    }

    (value, overflow)
}

fn c_string<'a>(memory: &'a [u8], start: usize) -> &'a [u8] {
    let tail = memory.get(start..).unwrap_or_default();
    let end = tail
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(tail.len());
    &tail[..end]
}

fn c_equals(memory: &[u8], start: usize, expected: &[u8]) -> bool {
    c_string(memory, start) == expected
}

fn prefix_equals(memory: &[u8], start: usize, expected: &[u8]) -> bool {
    memory
        .get(start..start.saturating_add(expected.len()))
        .is_some_and(|candidate| candidate == expected)
}

fn validate_token(memory: &[u8], reference: &[u8]) -> i32 {
    if c_equals(memory, 0, reference) {
        return 1;
    }
    if c_equals(memory, 0, b"VALID") || c_equals(memory, 0, b"OK") {
        return 1;
    }
    0
}

fn parse_command(memory: &[u8], input_len: usize) -> i32 {
    const COMMANDS: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];

    for (index, command) in COMMANDS.iter().enumerate() {
        if input_len >= command.len()
            && prefix_equals(memory, 0, command)
            && matches!(memory.get(command.len()), Some(0 | b' '))
        {
            return index as i32;
        }
        if c_equals(memory, 0, command) {
            return index as i32;
        }
    }

    if c_equals(memory, 0, b"ADMIN") {
        return 99;
    }
    -1
}

fn compare_prefix(memory: &[u8], reference: &[u8], exact_match: bool) -> i32 {
    if exact_match {
        if c_equals(memory, 0, reference) {
            return 1;
        }

        const VARIATIONS: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (index, variation) in VARIATIONS.iter().enumerate() {
            let prefix_len = reference.len().min(63);
            let mut expected = Vec::with_capacity(63);
            expected.extend_from_slice(&reference[..prefix_len]);
            let remaining = 63 - expected.len();
            expected.extend_from_slice(&variation[..variation.len().min(remaining)]);
            if c_equals(memory, 0, &expected) {
                return 2 + index as i32;
            }
        }
        0
    } else if prefix_equals(memory, 0, reference) {
        1
    } else {
        0
    }
}

fn find_delimiter(memory: &[u8], input_len: usize, delimiter: u8) -> i32 {
    if input_len == 0 {
        return -1;
    }

    for (index, &byte) in memory[..input_len].iter().enumerate() {
        if byte == delimiter {
            return index as i32;
        }
        if byte == 0 {
            break;
        }
    }

    if delimiter == b'|' && c_equals(memory, 0, b"NONE") {
        return -2;
    }
    if delimiter == b':' && c_equals(memory, 0, b"EMPTY") {
        return -3;
    }
    -1
}

fn snprintf_pattern(prefix: &[u8], pattern: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(63);
    for &byte in prefix.iter().chain(pattern).chain(suffix).take(63) {
        result.push(byte);
    }
    result
}

fn match_pattern(memory: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    let text = c_string(memory, 0);

    if case_sensitive {
        if text == pattern {
            return 1;
        }

        let wildcard_patterns = [
            snprintf_pattern(b"*", pattern, b"*"),
            snprintf_pattern(b"", pattern, b"*"),
            snprintf_pattern(b"*", pattern, b""),
        ];
        for (index, wildcard) in wildcard_patterns.iter().enumerate() {
            if text == wildcard {
                return 2 + index as i32;
            }
        }

        if pattern.len() <= text.len() {
            for index in 0..=text.len() - pattern.len() {
                if text[index..].starts_with(pattern) {
                    return 10_i32.wrapping_add(index as i32);
                }
            }
        } else {
            // The C size_t subtraction wraps here. Model reads through the
            // adjacent reference buffer while remaining inside known storage.
            for index in 0..memory.len() {
                if memory[index..].starts_with(pattern) {
                    return 10_i32.wrapping_add(index as i32);
                }
            }
        }
    } else {
        if text == pattern {
            return 1;
        }

        if text.len() != pattern.len() && text.starts_with(pattern) {
            return 5;
        }

        if text.len() == pattern.len()
            && text
                .iter()
                .zip(pattern)
                .all(|(&left, &right)| ascii_lower(left) == ascii_lower(right))
        {
            return 6;
        }
    }

    0
}

fn ascii_lower(byte: u8) -> u8 {
    if byte.is_ascii_uppercase() {
        byte + 32
    } else {
        byte
    }
}

fn process_strings(
    memory: &[u8],
    input_len: usize,
    ref_len: usize,
    operation: i32,
    flags: u32,
) -> i32 {
    let reference = c_string(memory, REFERENCE_OFFSET);

    match operation {
        0 => validate_token(memory, reference),
        1 => parse_command(memory, input_len),
        2 => compare_prefix(memory, reference, flags & 0x01 != 0),
        3 => {
            let delimiter = if ref_len > 0 {
                memory[REFERENCE_OFFSET]
            } else {
                b':'
            };
            find_delimiter(memory, input_len, delimiter)
        }
        4 => match_pattern(memory, reference, flags & 0x02 != 0),
        _ => -3,
    }
}

fn fail(message: &str) -> ! {
    let _ = io::stderr().write_all(message.as_bytes());
    std::process::exit(1);
}

fn main() {
    let mut scanner = Scanner::from_stdin();
    let operation = scanner
        .scan_i32()
        .unwrap_or_else(|| fail("Error reading operation\n"));
    let flags = scanner
        .scan_u32()
        .unwrap_or_else(|| fail("Error reading flags\n"));
    let input_len = scanner
        .scan_usize()
        .unwrap_or_else(|| fail("Error reading input length\n"));

    if input_len > MAX_BUFFER_SIZE {
        fail(&format!(
            "Error: input length {input_len} exceeds maximum {MAX_BUFFER_SIZE}\n"
        ));
    }

    let mut memory = [0_u8; MEMORY_SIZE];
    for index in 0..input_len {
        memory[index] = scanner
            .scan_u32()
            .unwrap_or_else(|| fail(&format!("Error reading input byte {index}\n")))
            as u8;
    }

    let ref_len = scanner
        .scan_usize()
        .unwrap_or_else(|| fail("Error reading reference length\n"));
    if ref_len > MAX_BUFFER_SIZE {
        fail(&format!(
            "Error: reference length {ref_len} exceeds maximum {MAX_BUFFER_SIZE}\n"
        ));
    }

    for index in 0..ref_len {
        memory[REFERENCE_OFFSET + index] = scanner
            .scan_u32()
            .unwrap_or_else(|| fail(&format!("Error reading reference byte {index}\n")))
            as u8;
    }

    let result = process_strings(&memory, input_len, ref_len, operation, flags);
    let _ = writeln!(io::stdout(), "{result}");
}
