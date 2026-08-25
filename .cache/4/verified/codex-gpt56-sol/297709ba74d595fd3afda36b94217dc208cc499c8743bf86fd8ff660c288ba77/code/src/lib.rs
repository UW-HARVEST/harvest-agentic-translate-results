use std::ffi::{c_char, c_int, CStr};

fn c_n_eq(left: &[u8], right: &[u8], count: usize) -> bool {
    for index in 0..count {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        if left_byte != right_byte {
            return false;
        }
        if left_byte == 0 {
            return true;
        }
    }
    true
}

fn validate_token(token: &[u8], expected: &[u8]) -> c_int {
    if token == expected || token == b"VALID" || token == b"OK" {
        1
    } else {
        0
    }
}

fn parse_command(buffer: &[u8], buf_size: usize) -> c_int {
    const COMMANDS: [&[u8]; 5] = [b"START", b"STOP", b"PAUSE", b"RESUME", b"RESET"];

    for (index, command) in COMMANDS.iter().enumerate() {
        let command_len = command.len();
        if buf_size >= command_len
            && c_n_eq(buffer, command, command_len)
            && matches!(buffer.get(command_len), None | Some(0 | b' '))
        {
            return index as c_int;
        }

        if buffer == *command {
            return index as c_int;
        }
    }

    if buffer == b"ADMIN" {
        99
    } else {
        -1
    }
}

fn compare_prefix(value: &[u8], prefix: &[u8], exact_match: bool) -> c_int {
    if exact_match {
        if value == prefix {
            return 1;
        }

        const VARIATIONS: [&[u8]; 5] = [b"_v1", b"_v2", b"_old", b"_new", b"_tmp"];
        for (index, variation) in VARIATIONS.iter().enumerate() {
            let mut expected = Vec::with_capacity(63);
            expected.extend_from_slice(&prefix[..prefix.len().min(63)]);
            let remaining = 63 - expected.len();
            expected.extend_from_slice(&variation[..variation.len().min(remaining)]);
            if value == expected {
                return 2 + index as c_int;
            }
        }

        0
    } else if c_n_eq(value, prefix, prefix.len()) {
        1
    } else {
        0
    }
}

unsafe fn find_delimiter(data: *const u8, len: usize, delimiter: u8) -> c_int {
    if len == 0 {
        return -1;
    }

    for index in 0..len {
        let byte = unsafe { data.add(index).read() };
        if byte == delimiter {
            return index as c_int;
        }
        if byte == 0 {
            break;
        }
    }

    let string = unsafe { CStr::from_ptr(data.cast()) }.to_bytes();
    if delimiter == b'|' && string == b"NONE" {
        -2
    } else if delimiter == b':' && string == b"EMPTY" {
        -3
    } else {
        -1
    }
}

fn truncated_pattern(prefix: &[u8], pattern: &[u8], suffix: &[u8]) -> Vec<u8> {
    prefix
        .iter()
        .chain(pattern)
        .chain(suffix)
        .take(63)
        .copied()
        .collect()
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> c_int {
    if case_sensitive {
        if text == pattern {
            return 1;
        }

        let wildcard_patterns = [
            truncated_pattern(b"*", pattern, b"*"),
            truncated_pattern(b"", pattern, b"*"),
            truncated_pattern(b"*", pattern, b""),
        ];
        for (index, wildcard) in wildcard_patterns.iter().enumerate() {
            if text == wildcard {
                return 2 + index as c_int;
            }
        }

        for index in 0..=text.len() - pattern.len() {
            if c_n_eq(&text[index..], pattern, pattern.len()) {
                return (10 + index) as c_int;
            }
        }
    } else {
        if text == pattern {
            return 1;
        }

        if text.len() != pattern.len() && c_n_eq(text, pattern, pattern.len()) {
            return 5;
        }

        if text.len() == pattern.len() {
            let matches = text
                .iter()
                .zip(pattern)
                .all(|(&left, &right)| left.to_ascii_lowercase() == right.to_ascii_lowercase());
            if matches {
                return 6;
            }
        }
    }

    0
}

unsafe fn c_bytes<'a>(pointer: *const c_char) -> &'a [u8] {
    unsafe { CStr::from_ptr(pointer) }.to_bytes()
}

/// C ABI entry point matching `c_src/src/lib.c::process_strings`.
///
/// # Safety
///
/// Non-null pointers must identify storage satisfying the same requirements as
/// the C implementation. Operations using C string functions require a NUL
/// terminator; operation 3 additionally requires `input_len` readable bytes.
#[no_mangle]
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
            if reference.is_null() {
                return -2;
            }
            validate_token(unsafe { c_bytes(input) }, unsafe { c_bytes(reference) })
        }
        1 => parse_command(unsafe { c_bytes(input) }, input_len),
        2 => {
            if reference.is_null() {
                return -2;
            }
            compare_prefix(
                unsafe { c_bytes(input) },
                unsafe { c_bytes(reference) },
                flags & 0x01 != 0,
            )
        }
        3 => {
            let delimiter = if !reference.is_null() && ref_len > 0 {
                unsafe { *reference.cast::<u8>() }
            } else {
                b':'
            };
            unsafe { find_delimiter(input.cast(), input_len, delimiter) }
        }
        4 => {
            if reference.is_null() {
                return -2;
            }
            match_pattern(
                unsafe { c_bytes(input) },
                unsafe { c_bytes(reference) },
                flags & 0x02 != 0,
            )
        }
        _ => -3,
    }
}
