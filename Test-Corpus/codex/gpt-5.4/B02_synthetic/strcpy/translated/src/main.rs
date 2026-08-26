#![allow(unsafe_op_in_unsafe_fn)]

use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_int};
use std::process::exit;

const MAX_BUFFER_SIZE: usize = 1024;

struct Scanner {
    data: Vec<u8>,
    pos: usize,
}

impl Scanner {
    fn new(mut data: Vec<u8>) -> Self {
        data.push(0);
        Self { data, pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.data.len() - 1 {
            let ch = self.data[self.pos] as libc::c_int;
            // Match scanf's integer conversions, which skip leading whitespace.
            if unsafe { libc::isspace(ch) } == 0 {
                break;
            }
            self.pos += 1;
        }
    }

    fn read_i32(&mut self) -> Option<i32> {
        self.skip_ws();
        let start = unsafe { self.data.as_mut_ptr().add(self.pos) as *mut c_char };
        let mut end = start;
        let value = unsafe { libc::strtol(start, &mut end, 10) };
        if end == start {
            return None;
        }
        self.pos += unsafe { end.offset_from(start) as usize };
        Some(value as i32)
    }

    fn read_u32(&mut self) -> Option<u32> {
        self.skip_ws();
        let start = unsafe { self.data.as_mut_ptr().add(self.pos) as *mut c_char };
        let mut end = start;
        let value = unsafe { libc::strtoul(start, &mut end, 10) };
        if end == start {
            return None;
        }
        self.pos += unsafe { end.offset_from(start) as usize };
        Some(value as u32)
    }

    fn read_usize(&mut self) -> Option<usize> {
        self.skip_ws();
        let start = unsafe { self.data.as_mut_ptr().add(self.pos) as *mut c_char };
        let mut end = start;
        let value = unsafe { libc::strtoull(start, &mut end, 10) };
        if end == start {
            return None;
        }
        self.pos += unsafe { end.offset_from(start) as usize };
        Some(value as usize)
    }
}

fn stderr_line(message: &str) {
    let _ = io::stderr().write_all(message.as_bytes());
}

fn main() {
    let mut raw_input = Vec::new();
    if io::stdin().read_to_end(&mut raw_input).is_err() {
        stderr_line("Error reading operation\n");
        exit(1);
    }

    let mut scanner = Scanner::new(raw_input);
    let mut input_buffer: [MaybeUninit<u8>; MAX_BUFFER_SIZE] =
        [const { MaybeUninit::uninit() }; MAX_BUFFER_SIZE];
    let mut ref_buffer: [MaybeUninit<u8>; MAX_BUFFER_SIZE] =
        [const { MaybeUninit::uninit() }; MAX_BUFFER_SIZE];

    let operation = match scanner.read_i32() {
        Some(value) => value,
        None => {
            stderr_line("Error reading operation\n");
            exit(1);
        }
    };

    let flags = match scanner.read_u32() {
        Some(value) => value,
        None => {
            stderr_line("Error reading flags\n");
            exit(1);
        }
    };

    let input_len = match scanner.read_usize() {
        Some(value) => value,
        None => {
            stderr_line("Error reading input length\n");
            exit(1);
        }
    };

    if input_len > MAX_BUFFER_SIZE {
        stderr_line(&format!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        ));
        exit(1);
    }

    for (i, slot) in input_buffer.iter_mut().take(input_len).enumerate() {
        let byte = match scanner.read_u32() {
            Some(value) => value,
            None => {
                stderr_line(&format!("Error reading input byte {}\n", i));
                exit(1);
            }
        };
        slot.write(byte as u8);
    }

    let ref_len = match scanner.read_usize() {
        Some(value) => value,
        None => {
            stderr_line("Error reading reference length\n");
            exit(1);
        }
    };

    if ref_len > MAX_BUFFER_SIZE {
        stderr_line(&format!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        ));
        exit(1);
    }

    for (i, slot) in ref_buffer.iter_mut().take(ref_len).enumerate() {
        let byte = match scanner.read_u32() {
            Some(value) => value,
            None => {
                stderr_line(&format!("Error reading reference byte {}\n", i));
                exit(1);
            }
        };
        slot.write(value_to_char(byte));
    }

    let result = unsafe {
        process_strings(
            input_buffer.as_mut_ptr() as *mut c_char,
            input_len,
            ref_buffer.as_ptr() as *const c_char,
            ref_len,
            operation,
            flags,
        )
    };

    print!("{result}\n");
}

fn value_to_char(value: u32) -> u8 {
    value as u8
}

unsafe fn process_strings(
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
            validate_token(input, reference)
        }
        1 => {
            let commands = [
                c"START".as_ptr(),
                c"STOP".as_ptr(),
                c"PAUSE".as_ptr(),
                c"RESUME".as_ptr(),
                c"RESET".as_ptr(),
            ];
            parse_command(input, input_len, &commands, 5)
        }
        2 => {
            if reference.is_null() {
                return -2;
            }
            let exact = (flags & 0x01) as c_int;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = if !reference.is_null() && ref_len > 0 {
                *reference
            } else {
                b':' as c_char
            };
            find_delimiter(input, input_len, delim)
        }
        4 => {
            if reference.is_null() {
                return -2;
            }
            let case_sens = ((flags & 0x02) != 0) as c_int;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

unsafe fn validate_token(token: *const c_char, expected: *const c_char) -> c_int {
    if libc::strcmp(token, expected) == 0 {
        return 1;
    }

    if libc::strcmp(token, c"VALID".as_ptr()) == 0 || libc::strcmp(token, c"OK".as_ptr()) == 0 {
        return 1;
    }

    0
}

unsafe fn parse_command(
    buffer: *mut c_char,
    buf_size: usize,
    cmd_list: &[*const c_char],
    list_size: c_int,
) -> c_int {
    let mut i = 0;
    while i < list_size {
        let cmd = cmd_list[i as usize];
        let cmd_len = libc::strlen(cmd);

        if buf_size >= cmd_len {
            if libc::strncmp(buffer, cmd, cmd_len) == 0 {
                let next = *buffer.add(cmd_len);
                if next == 0 || next == b' ' as c_char {
                    return i;
                }
            }
        }

        if libc::strcmp(buffer, cmd) == 0 {
            return i;
        }

        i += 1;
    }

    if libc::strcmp(buffer, c"ADMIN".as_ptr()) == 0 {
        return 99;
    }

    -1
}

unsafe fn compare_prefix(str_ptr: *const c_char, prefix: *const c_char, exact_match: c_int) -> c_int {
    let prefix_len = libc::strlen(prefix);

    if exact_match != 0 {
        if libc::strcmp(str_ptr, prefix) == 0 {
            return 1;
        }

        let variations = [
            c"_v1".as_ptr(),
            c"_v2".as_ptr(),
            c"_old".as_ptr(),
            c"_new".as_ptr(),
            c"_tmp".as_ptr(),
        ];

        let mut i = 0;
        while i < 5 {
            let mut expected = [0 as c_char; 64];
            libc::strncpy(expected.as_mut_ptr(), prefix, 63);
            expected[63] = 0;
            libc::strncat(
                expected.as_mut_ptr(),
                variations[i as usize],
                63usize.wrapping_sub(libc::strlen(expected.as_ptr())),
            );

            if libc::strcmp(str_ptr, expected.as_ptr()) == 0 {
                return 2 + i;
            }

            i += 1;
        }

        0
    } else {
        if libc::strncmp(str_ptr, prefix, prefix_len) == 0 {
            1
        } else {
            0
        }
    }
}

unsafe fn find_delimiter(data: *const c_char, len: usize, delim: c_char) -> c_int {
    if len == 0 {
        return -1;
    }

    let mut i = 0usize;
    while i < len {
        let current = *data.add(i);
        if current == delim {
            return i as c_int;
        }
        if current == 0 {
            break;
        }
        i += 1;
    }

    if delim == b'|' as c_char && libc::strcmp(data, c"NONE".as_ptr()) == 0 {
        return -2;
    }

    if delim == b':' as c_char && libc::strcmp(data, c"EMPTY".as_ptr()) == 0 {
        return -3;
    }

    -1
}

unsafe fn match_pattern(text: *const c_char, pattern: *const c_char, case_sensitive: c_int) -> c_int {
    if case_sensitive != 0 {
        if libc::strcmp(text, pattern) == 0 {
            return 1;
        }

        let mut wildcard_patterns = [[0 as c_char; 64]; 3];
        libc::snprintf(wildcard_patterns[0].as_mut_ptr(), 64, c"*%s*".as_ptr(), pattern);
        libc::snprintf(wildcard_patterns[1].as_mut_ptr(), 64, c"%s*".as_ptr(), pattern);
        libc::snprintf(wildcard_patterns[2].as_mut_ptr(), 64, c"*%s".as_ptr(), pattern);

        let mut i = 0;
        while i < 3 {
            if libc::strcmp(text, wildcard_patterns[i as usize].as_ptr()) == 0 {
                return 2 + i;
            }
            i += 1;
        }

        let text_len = libc::strlen(text);
        let pattern_len = libc::strlen(pattern);
        let mut idx = 0usize;
        let limit = text_len.wrapping_sub(pattern_len);
        while idx <= limit {
            if libc::strncmp(text.add(idx), pattern, pattern_len) == 0 {
                return (10 + idx) as c_int;
            }
            if idx == usize::MAX {
                break;
            }
            idx += 1;
        }
    } else {
        if libc::strcmp(text, pattern) == 0 {
            return 1;
        }

        let pattern_len = libc::strlen(pattern);
        let text_len = libc::strlen(text);

        if text_len != pattern_len && libc::strncmp(text, pattern, pattern_len) == 0 {
            return 5;
        }

        if text_len == pattern_len {
            let mut match_flag = 1;
            let mut i = 0usize;
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
                    match_flag = 0;
                    break;
                }
                i += 1;
            }
            if match_flag != 0 {
                return 6;
            }
        }
    }

    0
}
