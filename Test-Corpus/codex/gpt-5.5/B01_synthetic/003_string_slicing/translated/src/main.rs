use std::env;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

fn strtol_i32_prefix(bytes: &[u8]) -> (i32, bool) {
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }

    let mut negative = false;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            negative = true;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
        }
    }

    let digit_start = i;
    let mut value: i128 = 0;
    let limit = if negative {
        i128::from(i64::MAX) + 1
    } else {
        i128::from(i64::MAX)
    };
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = i128::from(bytes[i] - b'0');
        value = if value > (limit - digit) / 10 {
            limit
        } else {
            value * 10 + digit
        };
        i += 1;
    }

    if i == digit_start {
        return (0, false);
    }

    let signed = if negative { -value } else { value };
    let long_value = signed.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
    (long_value as i32, true)
}

fn greater_than_size_t(value: i32, len: usize) -> bool {
    if value < 0 {
        true
    } else {
        (value as usize) > len
    }
}

fn emit(bytes: &[u8]) {
    let mut stdout = io::stdout();
    stdout.write_all(bytes).unwrap();
    stdout.flush().unwrap();
}

fn run() -> u8 {
    let args: Vec<_> = env::args_os().collect();
    let argc = args.len();

    if (argc > 4) || (argc == 1) {
        emit(b"Error: there should be one to three arguments passed:\n<string> [start] [stop]\n");
        return 1;
    }

    let string = args[1].as_os_str().as_bytes();
    let len = string.len();

    let start: i32;
    let stop: i32;
    let mut second_had_digits = false;

    if argc >= 3 {
        let parsed = strtol_i32_prefix(args[2].as_os_str().as_bytes());
        start = parsed.0;
        second_had_digits = parsed.1;
        if !second_had_digits {
            emit(b"Second argument must be an integer!");
            return 1;
        }
        if greater_than_size_t(start, len) {
            emit(b"Error: start is off the end of the string!\n");
            return 1;
        }
    } else {
        start = 0;
    }

    if argc == 4 {
        stop = strtol_i32_prefix(args[3].as_os_str().as_bytes()).0;

        // The C source accidentally checks the end pointer from argv[2] here.
        // Since argv[2] was already validated, this branch is unreachable in
        // ordinary process argument layouts, including the original program.
        if !second_had_digits
            && args[2].as_os_str().as_bytes().as_ptr() == args[3].as_os_str().as_bytes().as_ptr()
        {
            emit(b"Third argument must be an integer!");
            return 1;
        }

        if greater_than_size_t(stop, len) {
            emit(b"Error: stop is off the end of the string!\n");
            return 1;
        }

        if stop <= start {
            emit(b"Error: stop must come after start!\n");
            return 1;
        }
    } else {
        stop = len as i32;
    }

    let start_usize = start as usize;
    let count = (stop - start) as usize;
    let end = start_usize + count;
    let mut stdout = io::stdout();
    stdout.write_all(&string[start_usize..end]).unwrap();
    stdout.write_all(b"\n").unwrap();
    stdout.flush().unwrap();
    0
}

fn main() -> ExitCode {
    ExitCode::from(run())
}
